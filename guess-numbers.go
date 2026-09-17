package main

import (
	"bufio"
	"fmt"
	"math/rand"
	"os"
	"strconv"
	"strings"
	"time"
)

func main() {
	rng := rand.New(rand.NewSource(time.Now().UnixNano()))
	secretNumber := rng.Intn(100) + 1

	scanner := bufio.NewScanner(os.Stdin)

	for {
		fmt.Print("Guess a number between 1 and 100: ")

		if !scanner.Scan() {
			fmt.Println()
			return
		}

		input := strings.TrimSpace(scanner.Text())
		userGuess, err := strconv.Atoi(input)
		if err != nil {
			fmt.Println("Enter a whole number dummy.")
			continue
		}

		if userGuess < 1 || userGuess > 100 {
			fmt.Println("Bro. BETWEEN 1 and 100.")
			continue
		}

		if userGuess < secretNumber {
			fmt.Println("Haha SUCKKERRRRR. Too low. Try again!")
			continue
		}

		if userGuess > secretNumber {
			fmt.Println("Haha SUCKKERRRRR. Too high. Try again!")
			continue
		}

		fmt.Printf("NO WAY you guessed the freakin number: %d\n", secretNumber)
		return
	}
}
