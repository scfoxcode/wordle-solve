use std::thread;
use std::io;
use std::time::{SystemTime};
use std::sync::mpsc;
use regex::Regex;

mod freq;
use freq::FrequencyDistribution;
use freq::FrequencySet;
use freq::Distribution;

// static WORD_FILE: &'static str = include_str!("./wordlist-debug.txt");
// The non raw wordlist is missing words like Slate
static WORD_FILE: &'static str = include_str!("./wordlist-raw.txt");
static ANSWERS_FILE: &'static str = include_str!("./answers.txt");

// This needs to be smarter, as in exists but not in pos X
#[derive(Clone)] 
struct CurrentInfo {
    pub excluded: Vec<char>,
    pub somewhere: Vec<char>,
    pub exists: [Option<char>; 5], 
    pub known: [Option<char>; 5]
}

impl CurrentInfo {
    fn new() -> Self {
        Self {
            excluded: Vec::new(),
            somewhere: Vec::new(),
            exists: [None; 5],
            known: [None; 5],
        }
    }
}

fn prompt_user_for_new_state() -> CurrentInfo {
    let mut info = CurrentInfo::new();
    let mut known_letters_raw = String::new();

    println!("Please provide information about the current game state");
    println!("All information should be separated by spaces...");
    println!("- Eg: For the letter 'a' in position 2, enter 2a");
    println!("- Eg: letter 'd' exists but not in position 4, enter !4d");
    println!("- Eg: If letter 'c' does not exist, enter !c");
    println!("Full example: 1r 4s !2e !x !p !3r !y");

    io::stdin().read_line(&mut known_letters_raw)
        .expect("failed to read known letters");

    let inputs: Vec<&str> = known_letters_raw.split_whitespace().collect();
    println!("Inputs {:?}", inputs);
    for input in inputs {
        let known = r"^([1-5])([a-z])";
        let somewhere = r"^\!([1-5])([a-z])";
        let exclude = r"^\!([a-z])";

        let mut regex = Regex::new(known).unwrap();

        // Ugh, this feels ugly. And so many horrible unwraps
        if let Some(captures) = regex.captures(input) {
            if let Some(position) = captures.get(1) {
                if let Some(letter) = captures.get(2) {
                    // This might crash into next week with all these unwraps
                    let pos: usize = position.as_str().parse::<usize>().unwrap() - 1;
                    let letter: &str = letter.as_str();
                    info.known[pos] = letter.chars().nth(0); // Sneaky double use of option here
                }
            }
        }

        regex = Regex::new(somewhere).unwrap();
        if let Some(captures) = regex.captures(input) {
            if let Some(position) = captures.get(1) {
                if let Some(letter) = captures.get(2) {
                    // This might crash into next week with all these unwraps
                    let pos: usize = position.as_str().parse::<usize>().unwrap() - 1;
                    let letter: &str = letter.as_str();
                    info.exists[pos] = letter.chars().nth(0); // Sneaky double use of option here
                    info.somewhere.push(letter.chars().nth(0).unwrap());
                }
            }
        }

        regex = Regex::new(exclude).unwrap();
        if let Some(captures) = regex.captures(input) {
            if let Some(letter) = captures.get(1) {
                let letter: &str = letter.as_str();
                info.excluded.push(letter.chars().nth(0).unwrap());
            }
        }
        
    }

    info
}

fn possible_remaining_answers(
    answers: &Vec<&'static str>,
    info: &CurrentInfo,
    ) -> Vec<&'static str> {

    let mut remaining: Vec<&'static str> = Vec::new();
    for answer in answers.iter() {
        if answer.to_string() == "slate" {
            println!("YOLO");
            println!("{:?}", info.excluded);
            println!("{:?}", info.exists);
            println!("{:?}", info.known);
        }
        let mut valid = true;
        for letter in info.excluded.iter() {
            if answer.contains(*letter) {
                valid = false;
                break;
            }
        }

        for letter in info.somewhere.iter() {
            if !answer.contains(*letter) {
                valid = false;
                break;
            }
        }

        // Check word doesn't contain a somewhere letter in a disallowed location
        for (i, letter) in answer.chars().enumerate() {
            if let Some(not_here) = info.exists[i] {
                if not_here == letter {
                    valid = false;
                    break;
                }
            }
        }

        for (i, letter) in answer.chars().enumerate() {
            if let Some(known) = info.known[i] {
                if known != letter {
                    valid = false;
                    break;
                }
            }
        }

        if valid {
            remaining.push(answer);
        }
    }
    remaining
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
        }

        sender.send(vec![first, second, third]).unwrap();
    })
}

fn best_guesses(
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

fn print_guesses(guesses: &Vec<Guess>) {
    let mut count = 0;
    println!("Best possible guesses");
    while count < 17 {
        if guesses.len() > count + 3 {
            println!(
                "{}, {}, {}, {}",
                guesses[count].guess,
                guesses[count + 1].guess,
                guesses[count + 2].guess,
                guesses[count + 3].guess,
            );
        }
        count += 4;
    }
}

fn main() {
    let start = SystemTime::now();
    let thread_count = 16;

    let words: Vec<&'static str> = get_words(); 
    let answers: Vec<&'static str> = get_answers(); 
    println!("Welcome to the Wordle solver\n");

    let top_results = best_guesses(thread_count, &words, &answers);
    println!("Our analysis shows that the following are all great starting guesses\n");
    print_guesses(&top_results);

    println!("\nPlay wordle using one of these, then enter the information you learn after each guess\n");

    let mut num_answers = 1000;
    let mut remaining_answers = answers;

    while num_answers > 0 {
        let state = prompt_user_for_new_state();
        remaining_answers =  possible_remaining_answers(&remaining_answers, &state);
        num_answers = remaining_answers.len();
        println!("Num Possible Answers {}", remaining_answers.len());
        for (i, ans) in remaining_answers.iter().enumerate() {
            if i >= 5 {
                println!("...\n");
                break;
            }
            println!("{}", *ans);
        }
        let results = best_guesses(thread_count, &words, &remaining_answers);
        print_guesses(&results);
        println!("");
    }

    
    match start.elapsed() {
        Ok(elapsed) => {
            println!("Time taken {}ms", elapsed.as_millis());
        }
        Err(e) => {
            println!("Error: {e:?}");
        }
    }


}
