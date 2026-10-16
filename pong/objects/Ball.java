package objects;

// IMPORTS
import java.awt.*;


public class Ball {
    int x = 100;
    int y = 100;
    int size = 20;
    int speedX = 3;
    int speedY = 3;
    Color color = Color.RED;

    // Constructor
    public Ball(int tempX, int tempY) {
        x = tempX;
        y = tempY;
    }

    // Called once per frame by the game loop
    public void update() {
        x += speedX;
        y += speedY;
    }

    public void draw(Graphics g) {
        g.setColor(color);
        g.fillOval(x, y, size, size);
    }

    // Bounce off the edges of a width x height screen
    public void bounceOffWalls(int width, int height) {
        // left or right wall
        if (x <= 0 || x + size >= width) {
            bounceX();
        }
        // top or bottom wall
        if (y <= 0 || y + size >= height) {
            bounceY();
        }
    }

    public void bounceX() {
        speedX *= -1;
    }

    public void bounceY() {
        speedY *= -1;
    }
}
