import java.util.Scanner;

public class GuessNumbers {
    public static void main(String[] args) {
        Scanner scan = new Scanner(System.in);
        
        int secretNumber = (int) (Math.random() * 100) + 1;
        int userGuess;

        do {
            System.out.print("Guess a number between 1 and 100: \n");

            while (!scan.hasNextInt()) {
                System.out.println("Please enter a whole number.");
                scan.next();
                System.out.print("Guess a number between 1 and 100: \n");
            }

            userGuess = scan.nextInt();
        } while (userGuess < 1 || userGuess > 100);

        if (userGuess == secretNumber) {
            System.out.println("NO WAY you guessed the freakin number: " + secretNumber);
        } else {
            System.out.println("Haha SUCKKERRRRR. You guessed it wrong! It was " + secretNumber \n + \n);
            System.out.println("Try again next time!");
        }
    }
}
