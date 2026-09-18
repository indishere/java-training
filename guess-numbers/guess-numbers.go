package main

import (
	"fmt"
	"math/rand"
	"strconv"
)

func main() {
	secretNumber := rand.Intn(100) + 1
	userGuess := 0

	for userGuess != secretNumber {
		fmt.Print("Guess a number between 1 and 100: ")

		var input string
		fmt.Scan(&input)

		number, err := strconv.Atoi(input)
		if err != nil {
			fmt.Println("Enter a whole number dummy.")
			continue
		}

		userGuess = number

		if userGuess < 1 || userGuess > 100 {
			fmt.Println("Bro. BETWEEN 1 and 100.")
		} else if userGuess < secretNumber {
			fmt.Println("Haha SUCKKERRRRR. Too low. Try again!")
		} else if userGuess > secretNumber {
			fmt.Println("Haha SUCKKERRRRR. Too high. Try again!")
		}
	}

	fmt.Println("NO WAY you guessed the freakin number:", secretNumber)
}
