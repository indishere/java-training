use std::cmp::Ordering;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

fn random_number() -> i32 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before Unix epoch");

    let mut value = duration.as_nanos() as u64 ^ duration.as_secs();
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;

    (value % 100 + 1) as i32
}

fn main() {
    let secret_number = random_number();

    loop {
        print!("Guess a number between 1 and 100: ");
        io::stdout().flush().expect("Failed to flush output");

        let mut input = String::new();
        let bytes_read = match io::stdin().read_line(&mut input) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("Enter a whole number dummy.");
                continue;
            }
        };

        if bytes_read == 0 {
            println!();
            return;
        }

        let user_guess: i32 = match input.trim().parse() {
            Ok(number) => number,
            Err(_) => {
                println!("Enter a whole number dummy.");
                continue;
            }
        };

        if !(1..=100).contains(&user_guess) {
            println!("Bro. BETWEEN 1 and 100.");
            continue;
        }

        match user_guess.cmp(&secret_number) {
            Ordering::Less => println!("Haha SUCKKERRRRR. Too low. Try again!"),
            Ordering::Greater => println!("Haha SUCKKERRRRR. Too high. Try again!"),
            Ordering::Equal => {
                println!("NO WAY you guessed the freakin number: {}", secret_number);
                return;
            }
        }
    }
}
