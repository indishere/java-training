package objects;

// IMPORTS
import java.awt.*;


public class Paddle {
    int x;
    int y;
    int width = 15;
    int height = 100;
    int speed = 10;
    Color color = Color.BLUE;

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

    // Getters: let classes outside "objects" read the paddle
    public int getX() { return x; }
    public void setX(int newX) { x = newX; }
    public int getY() { return y; }
    public int getHeight() { return height; }

    public void draw(Graphics g) {
        g.setColor(color);
        g.fillRect(x, y, width, height);
    }
}
