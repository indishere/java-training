package objects;

// IMPORTS
import java.awt.*;


public class Ball {
    private int x;
    private int y;
    private final int size = 50;
    private int speedX = 5;
    private int speedY = 5;
    private int bouncePause = 0; // frames to wait before bouncing vertically again

    // Constructor
    public Ball(int x, int y) {
        this.x = x;
        this.y = y;
    }

    // Called once per frame by the game loop
    public void update() {
        x += speedX;
        y += speedY;
        if (bouncePause > 0) {
            bouncePause--;
        }
    }

    public void draw(Graphics g) {
        g.setColor(Color.RED);
        g.fillRect(x, y, size, size);
    }

    // Bounce off the edges of a screenWidth x screenHeight area
    public void bounceOffWalls(int screenWidth, int screenHeight) {
        if (x < 0 || x > screenWidth - size) {
            speedX = -speedX;
        }
        if (y < 20 || y > screenHeight - size) { // 20 = title bar
            bounceY();
        }
    }

    // Bounce up and angle away from the paddle's center
    public void bounceOff(Paddle paddle) {
        Rectangle p = paddle.getBounds();
        if (p.intersects(x, y, size, size)) {
            bounceY();
            speedX = (x + size / 2 - (int) p.getCenterX()) / 10;
        }
    }

    private void bounceY() {
        if (bouncePause == 0) {
            speedY = -speedY;
            bouncePause = 20;
        }
    }
}
