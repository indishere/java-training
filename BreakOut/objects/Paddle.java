package objects;

// IMPORTS
import java.awt.*;


// The paddle moves with the arrow keys or the mouse.
public class Paddle {
    private static final int HEIGHT = 14;
    private static final int KEY_SPEED = 9;
    private static final int EDGE_GAP = 8;

    private final int width = 110;
    private double x;       // middle of the paddle
    private final int y;
    private boolean movingLeft;
    private boolean movingRight;
    private double mouseTarget = -1;


    public Paddle(int centerX, int y) {
        x = centerX;
        this.y = y;
    }

    // Called once per frame by the game loop
    public void update(int screenWidth) {
        if (mouseTarget >= 0) {
            double distance = mouseTarget - x;
            if (Math.abs(distance) <= KEY_SPEED) {
                x = mouseTarget;
            } else {
                x += Math.signum(distance) * KEY_SPEED;
            }
        } else {
            if (movingLeft) x -= KEY_SPEED;
            if (movingRight) x += KEY_SPEED;
        }

        double half = width / 2.0;
        if (x - half < EDGE_GAP) x = EDGE_GAP + half;
        if (x + half > screenWidth - EDGE_GAP) x = screenWidth - EDGE_GAP - half;
    }

    public void draw(Graphics g) {
        g.setColor(Color.WHITE);
        g.fillRect((int) x - width / 2, y, width, HEIGHT);
    }

    public void moveLeft(boolean on) {
        movingLeft = on;
        if (on) mouseTarget = -1;
    }

    public void moveRight(boolean on) {
        movingRight = on;
        if (on) mouseTarget = -1;
    }

    public void followMouse(int screenX) {
        mouseTarget = screenX;
    }

    public void centerOn(int screenX) {
        x = screenX;
    }

    public Rectangle getBounds() {
        return new Rectangle((int) x - width / 2, y, width, HEIGHT);
    }

    public int getWidth() { return width; }
    public double getCenterX() { return x; }
}
