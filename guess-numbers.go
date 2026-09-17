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
    rand.Seed(time.Now().UnixNano())
    secretNumber := rand.Intn(100) + 1

    reader := bufio.NewReader(os.Stdin)

    for {
        fmt.Print("Guess a number between 1 and 100: ")

        input, err := reader.ReadString('\n')
        if err != nil {
            fmt.Println("Please enter a whole number.")
            continue
        }

        input = strings.TrimSpace(input)
        guess, err := strconv.Atoi(input)
        if err != nil {
            fmt.Println("Please enter a whole number.")
            continue
        }

        if guess < 1 || guess > 100 {
            fmt.Println("Please enter a number between 1 and 100.")
            continue
        }

        if guess == secretNumber {
            fmt.Printf("NO WAY you guessed the freakin number: %d\n", secretNumber)
            return
        } else if guess < secretNumber {
            fmt.Printf("Haha SUCKKERRRRR. Your guess is too low. You guessed it wrong! The number was %d\n", secretNumber)
        } else {
            fmt.Printf("Haha SUCKKERRRRR. Your guess is too high. You guessed it wrong! The number was %d\n", secretNumber)
        }

        fmt.Println("Try again! Guess a number between 1 and 100:")
    }
}
