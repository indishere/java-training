use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

fn random_number() -> i32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before Unix epoch")
        .as_nanos() as u64;

    (nanos % 100) as i32 + 1
}

fn main() {
    let secret_number = random_number();

    loop {
        print!("Guess a number between 1 and 100: ");
        io::stdout().flush().expect("Failed to flush output");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");

        let guess: i32 = match input.trim().parse() {
            Ok(number) => number,
            Err(_) => {
                println!("Please enter a whole number.");
                continue;
            }
        };

        if guess < 1 || guess > 100 {
            println!("Please enter a number between 1 and 100.");
            continue;
        }

        if guess == secret_number {
            println!("NO WAY you guessed the freakin number: {}", secret_number);
            break;
        } else if guess < secret_number {
            println!(
                "Haha SUCKKERRRRR. Your guess is too low. You guessed it wrong! The number was {}",
                secret_number
            );
        } else {
            println!(
                "Haha SUCKKERRRRR. Your guess is too high. You guessed it wrong! The number was {}",
                secret_number
            );
        }
        println!("Try again! Guess a number between 1 and 100:");
    }
}
