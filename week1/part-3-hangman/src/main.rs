// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    // println!("random word: {}", secret_word);

    // Your code here! :)
    let wordlen: usize = secret_word_chars.len();
    let mut current_word_chars: Vec<char> = "-".repeat(wordlen).chars().collect();
    let mut left_times: u32 = NUM_INCORRECT_GUESSES;
    let mut guess_right: usize = 0;
    let mut guessed_letters: String = String::new();
    println!("Welcome to CS110L Hangman!");
    while left_times > 0 && guess_right < wordlen {
        print!("The word so far is ");
        for ch in current_word_chars.iter() {
            print!("{}", ch);
        }
        println!();
        println!("You have guessed the following letters: {}", guessed_letters);
        println!("You have {} guesses left", left_times);
        print!("Please guess a letter: ");
        io::stdout()
            .flush()
            .expect("Error flushing stdout.");
        let mut guess = String::new();
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");
        if guess.len() > 3 {
            eprintln!("Please input a letter!")
        }
        guessed_letters.insert(guessed_letters.len(), guess.chars().nth(0).unwrap());
        let mut flag: bool = true;
        for i in 0..wordlen {
            if current_word_chars[i] == '-' && secret_word_chars[i] == guess.chars().nth(0).unwrap() {
                current_word_chars[i] = secret_word_chars[i];
                guess_right += 1;
                flag = false;
                break;
            }
        }
        if flag {
            println!("Sorry, that letter is not in the word");
            left_times -= 1;
        }
        println!();
    }
    if left_times == 0 {
        println!("Sorry, you ran out of guesses!");
    } else {
        println!("Congratulations you guessed the secret word: {}!", secret_word);
    }
}
