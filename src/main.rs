use std::thread;
use std::io;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use std::sync::mpsc;

mod freq;
use freq::FrequencyDistribution;
use freq::FrequencySet;
use freq::Distribution;

// static WORD_FILE: &'static str = include_str!("./wordlist-debug.txt");
// The non raw wordlist is missing words like Slate
static WORD_FILE: &'static str = include_str!("./wordlist-raw.txt");
static ANSWERS_FILE: &'static str = include_str!("./answers.txt");

fn prompt_user_for_new_state() {
    let mut known_letters_raw = String::new();
    println!(
        "Please provide known letter \
        locations eg: 1a !4b. This means \
        we have the letter 'a' in slot 1, \
        and we have the letter 'b', but not in slot 4");

    io::stdin().read_line(&mut known_letters_raw)
        .expect("failed to read known letters");

    println!("The input: {}", known_letters_raw);
}

// Returns a list of possible words it could still be for a given guess
// Ignore correct letter, wrong position for now
fn valid_remaining(guess: &str, answer: &str, words: &Vec<&str>) -> usize {
    let mut count: usize = 0;
    let mut correct_letters: [Option<char>; 5] = [None; 5];

    for i in 0..5 {
        if guess.chars().nth(i).unwrap() == answer.chars().nth(i).unwrap() {
            correct_letters[i] = Some(guess.chars().nth(i).unwrap());
        } else {
            correct_letters[i] = None;
        }
    }

    // To optimise this, take advantage of words being alphabetical
    // And then check first letter to only iterate over correct subset
    for i in 0..words.len() {
        let word = words[i as usize];
        let mut valid = true;
        for j in 0..5 {
            if let Some(letter) = correct_letters[j]  {
                if letter != word.chars().nth(j).unwrap() {
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            count += 1;
        }
    }

    count
}

#[derive(Clone, PartialOrd, PartialEq)] 
struct Guess {
    pub guess: String,
    pub score: f32,
}

impl Guess {
    pub fn new() -> Self {
        Self {
            guess: "NO GUESS".to_string(),
            score: 0.0,
        }
    }

    fn print(&self) {
        println!(
            "Word: {} Score: {}", 
            self.guess,
            self.score,
        );
    }
}

fn get_words() -> Vec<&'static str> {
    let mut words: Vec<&'static str> = WORD_FILE.split("\n").collect();
    words.pop();
    words
}

fn get_answers() -> Vec<&'static str> {
    let mut words: Vec<&'static str> = ANSWERS_FILE.split("\n").collect();
    words.pop();
    words
}

fn create_worker(
    words: Vec<&'static str>,
    answers: Vec<&'static str>,
    chunk: usize,
    num_chunks: usize,
    sender: mpsc::Sender<Vec<Guess>>) -> thread::JoinHandle<()> {


    let chunk_size = words.len() / num_chunks;


    // Window into larger words list
    let window = words[(chunk * chunk_size)..(chunk_size * (chunk+1))].to_vec();


    thread::spawn(move || {
        let mut first = Guess::new();
        let mut second = Guess::new();
        let mut third = Guess::new();

        let mut frequency = FrequencySet::new();
        frequency.buildSetFromWords(&answers);

        // Guess index is for the window. so this is wrong!!!
        for guess in window.iter() {
            // Calculate the score for guess
            let mut specific: f32 = 0.0;
            let mut general: f32 = 0.0;

            let mut duplicates = FrequencyDistribution::new();
            for (j, char) in guess.chars().enumerate() { // 10 and 5
                let is_duplicate = duplicates.charCount(char) > 0;
                let score_specific = if is_duplicate { 6.0 } else { 15.0 };
                let score_general = if is_duplicate { 4.0 } else { 12.0 };
                                                         
                match frequency.distributionForIndex(j) {
                    Ok(dist) => {
                        specific += dist.charFrequency(char) * score_specific; // 26 magic number heuristic
                    },
                    Err(err) => {
                        panic!("{}", err);
                    }
                }

                general += frequency.charFrequency(char) * score_general;
                duplicates.incrementChar(char);
            }

            let total = specific + general;
            // Ugly but simple
            if first.score < total {
                third = second;
                second = first;
                first = Guess::new();
                first.score = total;
                first.guess = guess.to_string();
            } else if second.score < total {
                third = second;
                second = Guess::new();
                second.score = total;
                second.guess = guess.to_string();
            } else if third.score < total {
                third.score = total;
                third.guess = guess.to_string();
            }

            /*

            // Check specific guess
            if guess.to_string() == "slate" {
                println!("Slate score = {}", total);
            }
            if guess.to_string() == "arise" {
                println!("Arise score = {}", total);
            }
            if guess.to_string() == "salet" {
                println!("Salet score = {}", total);
            }
            if guess.to_string() == "soare" {
                println!("Soare score = {}", total);
            }
            */
        }

        sender.send(vec![first, second, third]).unwrap();
    })
}

fn best_starting_guesses(
    thread_count: i32,
    words: &Vec<&'static str>,
    answers: &Vec<&'static str>
    ) -> Vec<Guess> {
    let (tx, rx): (mpsc::Sender<Vec<Guess>>, mpsc::Receiver<Vec<Guess>>) = mpsc::channel();
    let mut handles = vec![];

    for i in 0..thread_count {
        let tx_clone = tx.clone();
        let words_clone = words.clone();
        let answers_clone = answers.clone();
        handles.push(
            create_worker(
                words_clone,
                answers_clone,
                i as usize,
                thread_count as usize,
                tx_clone
            )
        );
    }

    // Results from threads
    let mut top_results: Vec<Guess> = Vec::new();
    for _ in 0..thread_count {
        let mut quality = rx.recv().unwrap();
        top_results.append(&mut quality);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    top_results.sort_by(|a, b| b.score.partial_cmp(&a.score).expect("I HATE FLOATS"));
    top_results
}

fn main() {
    let start = SystemTime::now();
    let thread_count = 16;

    let words: Vec<&'static str> = get_words(); 
    let answers: Vec<&'static str> = get_answers(); 
    println!("Welcome to the Wordle solver\n");

    let top_results = best_starting_guesses(thread_count, &words, &answers);
    println!("Our analysis shows that the following are all great starting guesses\n");
    let mut count = 0;
    while count < 17 {
        if top_results.len() > count {
            println!(
                "{}, {}, {}, {}",
                top_results[count].guess,
                top_results[count + 1].guess,
                top_results[count + 2].guess,
                top_results[count + 3].guess,
            );
        }
        count += 4;
    }

    println!("\nPlay wordle using one of these, then enter the information you learn after each guess");
    

    
    match start.elapsed() {
        Ok(elapsed) => {
            println!("Time taken {}ms", elapsed.as_millis());
        }
        Err(e) => {
            println!("Error: {e:?}");
        }
    }


}
