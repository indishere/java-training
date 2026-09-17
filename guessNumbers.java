import java.util.Scanner;

public class guessNumbers {
    public static void main(String[] args) {
        Scanner scan = new Scanner(System.in);
        
        int secretNumber = (int) (Math.random() * 100) + 1;
        int userGuess;

        do {
            System.out.print("Guess a number between 1 and 100: \n");
            while (!scan.hasNextInt()) {
                System.out.println("Enter a whole number dummy.");
                scan.next();
                System.out.print("Guess a number between 1 and 100: \n");
            }

            userGuess = scan.nextInt();
        } while (userGuess < 1 || userGuess > 100);

        if (userGuess == secretNumber) {
            System.out.println("NO WAY you guessed the freakin number: " + secretNumber);
        } else if (userGuess < secretNumber) {
            System.out.println("Haha SUCKKERRRRR. Your guess is too low. You guessed it wrong!  The number was " + secretNumber + "\n + \n");
            System.out.println("Try again! Guess a number between 1 and 100: \n");
            } else if (userGuess > secretNumber) {
                System.out.println("Haha SUCKKERRRRR. Your guess is too high. You guessed it wrong!  The number was " + secretNumber + "\n + \n");
                System.out.println("Try again! Guess a number between 1 and 100: \n");
            }
        }
    }
