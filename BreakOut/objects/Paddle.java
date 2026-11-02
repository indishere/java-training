package objects;

// IMPORTS
import java.awt.*;


public class Paddle {
    private static final int SPEED = 5;
    private int x;
    private final int y;
    private final int width = 100;
    private final int height = 20;
    private final int maxX;
    private int dir = 0;

    // Constructor: screenWidth is the area the paddle must stay inside
    public Paddle(int x, int y, int screenWidth) {
        this.x = x;
        this.y = y;
        maxX = screenWidth - width;
    }

    // Called once per frame by the game loop
    public void update() {
        x = Math.max(0, Math.min(maxX, x + dir));
    }

    public void draw(Graphics g) {
        g.setColor(Color.WHITE);
        g.fillRect(x, y, width, height);
    }

    public Rectangle getBounds() {
        return new Rectangle(x, y, width, height);
    }

    public void moveRight() {
        dir = SPEED;
    }

    public void moveLeft() {
        dir = -SPEED;
    }

    public void stopRight() {
        if (dir > 0) dir = 0;
    }

    public void stopLeft() {
        if (dir < 0) dir = 0;
    }
}
