use std::thread;
use std::io;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use std::sync::mpsc;

// static WORD_FILE: &'static str = include_str!("./wordlist-debug.txt");
static WORD_FILE: &'static str = include_str!("./wordlist.txt");

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

#[derive(Clone)]
struct GuessQuality {
    pub guess_index: usize,
    pub average: usize,
    pub lowest: usize,
    pub highest: usize 
}

impl GuessQuality {
    fn new() -> Self {
        Self {
            guess_index: 0,
            average: 9999999,
            lowest: 9999999,
            highest: 0,
        }
    }

    fn print(&self, words: &Vec<&str>) {
        println!(
            "Word: {} Lowest: {} Average: {} Highest: {}", 
            words[self.guess_index],
            self.lowest,
            self.average,
            self.highest
        );
    }
}

fn get_words() -> Vec<&'static str> {
    let mut words: Vec<&'static str> = WORD_FILE.split("\n").collect();
    words.pop();

    let mut final_words: Vec<&str> = Vec::new();
    let mut count = 0;
    for word in words.iter() {
        count += 1;
        if count % 2 == 1 {
            continue;
        }
        let mut chars_in_word: HashMap<char, bool> = HashMap::new();
        let mut valid = true;
        for i in 0..5 {
            let syb = word.chars().nth(i).unwrap();
            if chars_in_word.contains_key(&syb) {
                valid = false;
                break;
            } 
            chars_in_word.insert(syb, true);
        }
        if valid {
            final_words.push(word);
        }
    }

    final_words 
}

fn create_worker(raw_words: Vec<&'static str>, chunk: usize, num_chunks: usize, sender: mpsc::Sender<GuessQuality>) -> thread::JoinHandle<()> {
    let words = raw_words.clone();

    let chunk_size = words.len() / num_chunks;


    // Window into larger words list
    let window = raw_words[(chunk * chunk_size)..(chunk_size * (chunk+1))].to_vec();


    thread::spawn(move || {
        let mut first = GuessQuality::new();

        for (i, guess) in window.iter().enumerate() {
            let mut quality = GuessQuality::new();
            quality.guess_index = i;

            //for j in 0..words.len() {
                //let answer = words[j];
            for answer in &words {
                let result = valid_remaining(guess, answer, &words);            
                if result < quality.lowest {
                    quality.lowest = result;
                }
                if result > quality.highest {
                    quality.highest = result;
                }
                quality.average += result;
            }
            // quality.average = quality.average / (words.len() * words.len());
            // quality.average = quality.average; // The actual avg doesn't even matter. Lowest sum wins
            quality.average = quality.average / 10000;
            if quality.average < first.average {
                first = quality.clone();
            }
        }

        sender.send(first).unwrap();
    })
}

fn main() {
    let start = SystemTime::now();
    let thread_count = 8;

    let words: Vec<&'static str> = get_words(); 
    println!("Number of words in reduced list {}", words.len());
    println!("Beginning Brutal calculation");

    let (tx, rx): (mpsc::Sender<GuessQuality>, mpsc::Receiver<GuessQuality>) = mpsc::channel();
    let mut handles = vec![];
    for i in 0..thread_count {
        let tx_clone = tx.clone();
        let words_clone = words.clone();
        handles.push(
            create_worker(
                words_clone,
                i,
                thread_count,
                tx_clone
            )
        );
    }

    // Results from threads
    for _ in 0..thread_count {
        let quality = rx.recv().unwrap();
        quality.print(&words);
    }
    
    for handle in handles {
        handle.join().unwrap();
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
