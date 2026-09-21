import kotlin.random.Random

fun main() {
    val secretNumber = Random.nextInt(1, 101)
    var userGuess = 0

    do {
        print("Guess a number between 1 and 100: ")

        // toIntOrNull returns null instead of crashing when the input is not a whole number.
        val input = readLine()
        val guessedNumber = input?.trim()?.toIntOrNull()

        if (guessedNumber == null) {
            println("Enter a whole number dummy.")
            continue
        }

        userGuess = guessedNumber

        if (userGuess < 1 || userGuess > 100) {
            println("Bro. BETWEEN 1 and 100.")
        } else if (userGuess < secretNumber) {
            println("Haha SUCKKERRRRR. Too low. Try again!")
        } else if (userGuess > secretNumber) {
            println("Haha SUCKKERRRRR. Too high. Try again!")
        }
    } while (userGuess != secretNumber)

    println("NO WAY you guessed the freakin number: $secretNumber")
}
