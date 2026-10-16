package objects;

// IMPORTS
import java.awt.*;


public class Paddle {
    int x;
    int y;
    int width = 15;
    int height = 100;
    int speed = 6;
    Color color = Color.WHITE;

    // Set by the keyboard in Game
    public boolean movingUp = false;
    public boolean movingDown = false;

    // Constructor
    public Paddle(int tempX, int tempY) {
        x = tempX;
        y = tempY;
    }

    // Called once per frame by the game loop
    public void update(int screenHeight) {
        if (movingUp) {
            y = y - speed;
        }
        if (movingDown) {
            y = y + speed;
        }

        // keep the paddle on screen
        if (y < 0) {
            y = 0;
        }
        if (y > screenHeight - height) {
            y = screenHeight - height;
        }
    }

    public void draw(Graphics g) {
        g.setColor(color);
        g.fillRect(x, y, width, height);
    }
}
