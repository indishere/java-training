// IMPORTS
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import objects.Ball;
import objects.Paddle;


// The engine plays the right paddle.
//
// How it works:
//  1. DEFEND: when the ball starts coming towards the right paddle, Java works
//     out exactly where it will arrive (bounces included) and the paddle
//     starts moving there straight away. Instant and free.
//  2. ATTACK: at the same time it asks Cloudflare's clef-flash model ONCE:
//     "which way should I send the ball so the player can't reach it?"
//     The paddle then shifts a little so the ball hits its top, middle or
//     bottom, which sends the ball up, straight or down.
//
// It "extends Thread" because asking the internet takes ~0.5s, and the game
// draws a frame every 0.016s. If the game loop waited, the game would freeze.
public class Engine extends Thread {
    // Use Windows' trusted certificates instead of Java's own list.
    // Needed on networks (like school) that inspect HTTPS traffic,
    // otherwise you get "PKIX path building failed".
    static {
        if (System.getProperty("os.name").startsWith("Windows")) {
            System.setProperty("javax.net.ssl.trustStoreType", "Windows-ROOT");
        }
    }

    // How far from the paddle's middle to hit the ball when aiming up or down
    static final int AIM_SHIFT = 35;

    Game game;
    String apiKey = "";
    String accountId = "";
    HttpClient client = HttpClient.newHttpClient();

    // Where the paddle's middle should go.
    // "volatile" = safe to read from the game loop while this thread writes it.
    volatile int targetY;

    // Constructor
    public Engine(Game tempGame) {
        game = tempGame;
        targetY = game.height / 2;
        loadEnv(".env");
        setDaemon(true); // stop this thread when the game window closes
    }

    // Read KEY=value lines from the .env file
    void loadEnv(String fileName) {
        try {
            List<String> lines = Files.readAllLines(Path.of(fileName));
            for (String line : lines) {
                String[] parts = line.split("=", 2); // split at the first "="
                if (parts.length < 2) {
                    continue; // skip empty or broken lines
                }
                String key = parts[0].trim();
                String value = parts[1].trim();

                if (key.equals("CLOUDFLARE_CLEF_API_KEY")) {
                    apiKey = value;
                }
                if (key.equals("CLOUDFLARE_ACCOUNT_ID")) {
                    accountId = value;
                }
            }
        } catch (Exception e) {
            System.out.println("Engine: couldn't read " + fileName + ": " + e.getMessage());
        }
    }

    // Called every frame by Game: move the paddle towards targetY
    public void steer(Paddle paddle) {
        int paddleMiddle = paddle.getY() + paddle.getHeight() / 2;
        double speed = game.ball.getSpeed(); // paddle speed, stops it wobbling
        paddle.movingUp = paddleMiddle > targetY + speed;
        paddle.movingDown = paddleMiddle < targetY - speed;
    }

    // This is what runs when you call engine.start()
    @Override
    public void run() {
        boolean modelOn = !apiKey.equals("") && !accountId.equals("");
        if (!modelOn) {
            System.out.println("Engine: no keys in pong/.env, defending only (no aiming)");
        }

        boolean alreadyPlanned = false;

        while (true) {
            try {
                Ball ball = game.ball; // grab it once, reset() can swap it any time
                boolean comingAtMe = game.state.equals("playing") && ball.getSpeedX() > 0;

                if (comingAtMe && !alreadyPlanned) {
                    alreadyPlanned = true; // only plan (and call the model) ONCE per approach

                    // 1. DEFEND: go to where the ball will arrive, right now
                    int landingY = predictLandingY(ball, game.rightPaddle.getX());
                    targetY = landingY;

                    // 2. ATTACK: ask the model which way to send it back
                    if (modelOn) {
                        int playerMiddle = game.leftPaddle.getY() + game.leftPaddle.getHeight() / 2;
                        String shot = askModel(landingY, playerMiddle);

                        // only shift if there's still time before the ball arrives
                        if (game.ball == ball && framesUntilArrival(ball) > 15) {
                            if (shot.equals("up")) {
                                targetY = landingY + AIM_SHIFT; // ball hits the top part
                            }
                            if (shot.equals("down")) {
                                targetY = landingY - AIM_SHIFT; // ball hits the bottom part
                            }
                        }
                    }
                } else if (!comingAtMe) {
                    alreadyPlanned = false;
                    targetY = game.height / 2; // wait in the middle
                }

                Thread.sleep(10); // just checking the game here, no internet
            } catch (Exception e) {
                System.out.println("Engine error: " + e.getMessage());
                try {
                    Thread.sleep(2000); // wait before trying again
                } catch (InterruptedException ignored) {
                }
            }
        }
    }

    // Pretend to move a copy of the ball until it reaches the paddle,
    // bouncing off the top and bottom just like the real one
    int predictLandingY(Ball ball, int paddleX) {
        double x = ball.getX();
        double y = ball.getY();
        double speedX = ball.getSpeedX();
        double speedY = ball.getSpeedY();
        int size = ball.getSize();

        while (x + size < paddleX) {
            x += speedX;
            y += speedY;
            if (y <= 0 && speedY < 0) {
                speedY = -speedY;
            }
            if (y + size >= game.height && speedY > 0) {
                speedY = -speedY;
            }
        }
        return (int) y + size / 2; // middle of the ball
    }

    // How many frames until the ball reaches the right paddle
    double framesUntilArrival(Ball ball) {
        double distance = game.rightPaddle.getX() - (ball.getX() + ball.getSize());
        return distance / ball.getSpeedX();
    }

    // Ask clef-flash which way to send the ball: "up", "straight" or "down"
    String askModel(int hitY, int playerMiddle) throws Exception {
        // 1. Build the request body (JSON). """ is a text block: a multi-line string.
        String body = """
            {
              "model": "clef-flash",
              "state": "Pong. The screen is %d tall, y=0 is the top. You are the right paddle and you are about to hit the ball back at y=%d. The other player's paddle is on the left side, centred at y=%d. The ball bounces off the top and bottom walls.",
              "questions": {
                "shot": {
                  "type": "choice",
                  "instructions": "Which way should you send the ball so it is hardest for the other player to reach? Aim away from where they are.",
                  "criteria": {
                    "up": "Send the ball upward, towards y=0",
                    "straight": "Send the ball straight across",
                    "down": "Send the ball downward, towards the bottom"
                  }
                }
              }
            }
            """.formatted(game.height, hitY, playerMiddle);

        // 2. Send it
        String url = "https://api.cloudflare.com/client/v4/accounts/" + accountId
                + "/ai/run/@cf/cloudflare/clef-flash";
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(url))
                .header("Authorization", "Bearer " + apiKey)
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(body))
                .build();
        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());

        if (response.statusCode() != 200) {
            throw new Exception("HTTP " + response.statusCode() + ": " + response.body());
        }

        // 3. Pull the answer out of the reply, which looks like:
        //    ..."choice":"down","probabilities":{...},"confidence":0.4...
        // ponytail: regex instead of a JSON library, swap in Gson/Jackson if replies get complicated
        Matcher choice = Pattern.compile("\"choice\"\\s*:\\s*\"(\\w+)\"").matcher(response.body());
        if (choice.find()) {
            return choice.group(1);
        }
        throw new Exception("Weird reply: " + response.body());
    }
}
