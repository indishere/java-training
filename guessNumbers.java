import java.util.Scanner;

public class guessNumbers {
    public static void main(String[] args) {
        Scanner scan = new Scanner(System.in);

        int secretNumber = (int) (Math.random() * 100) + 1;
        int userGuess;

        do {
            System.out.print("Guess a number between 1 and 100: ");

            while (!scan.hasNextInt()) {
                System.out.println("Enter a whole number dummy.");
                scan.next();
                System.out.print("Guess a number between 1 and 100: ");
            }

            userGuess = scan.nextInt();

            if (userGuess < 1 || userGuess > 100) {
                System.out.println("Bro. BETWEEN 1 and 100.");
            } else if (userGuess < secretNumber) {
                System.out.println("Haha SUCKKERRRRR. Too low. Try again!");
            } else if (userGuess > secretNumber) {
                System.out.println("Haha SUCKKERRRRR. Too high. Try again!");
            }
        } while (userGuess != secretNumber);

        System.out.println("NO WAY you guessed the freakin number: " + secretNumber);
        scan.close();
    }
}
