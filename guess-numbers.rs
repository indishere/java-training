use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let secret_number = (time % 100) as i32 + 1;
    let mut user_guess = 0;

    while user_guess != secret_number {
        println!("Guess a number between 1 and 100:");

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        user_guess = match input.trim().parse() {
            Ok(number) => number,
            Err(_) => {
                println!("Enter a whole number dummy.");
                continue;
            }
        };

        if user_guess < 1 || user_guess > 100 {
            println!("Bro. BETWEEN 1 and 100.");
        } else if user_guess < secret_number {
            println!("Haha SUCKKERRRRR. Too low. Try again!");
        } else if user_guess > secret_number {
            println!("Haha SUCKKERRRRR. Too high. Try again!");
        }
    }

    println!("NO WAY you guessed the freakin number: {}", secret_number);
}
