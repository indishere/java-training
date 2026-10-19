package objects;

// IMPORTS
import java.awt.*;


public class Ball {
    // double = a number with decimals, so the speed can grow by small steps
    double x = 100;
    double y = 100;
    int size = 30;
    double speedX = 4.5;
    double speedY = 4.5;
    double speedUp = 1.10; // 10% faster on every paddle hit
    Color color = Color.RED;

    // Constructor
    public Ball(int tempX, int tempY) {
        x = tempX;
        y = tempY;

        // 50% chance to go left instead of right
        if (Math.random() < 0.5) {
            speedX = -speedX;
        }
    }

    // Called once per frame by the game loop
    public void update() {
        x += speedX;
        y += speedY;
    }

    public void draw(Graphics g) {
        g.setColor(color);
        g.fillOval((int) x, (int) y, size, size); // (int) = drop the decimals
    }

    // Bounce off the top and bottom of the screen
    public void bounceOffWalls(int height) {
        // only bounce if moving into the wall, so a fast ball can't get stuck in it
        if (y <= 0 && speedY < 0) {
            bounceY();
        }
        if (y + size >= height && speedY > 0) {
            bounceY();
        }
    }

    // True if the ball touched the left or right wall
    public boolean hitSideWall(int width) {
        return x <= 0 || x + size >= width;
    }

    // Bounce off a paddle if the ball is touching it
    public void bounceOffPaddle(Paddle paddle) {
        boolean touching = x < paddle.x + paddle.width
                        && x + size > paddle.x
                        && y < paddle.y + paddle.height
                        && y + size > paddle.y;

        if (touching) {
            // only bounce if the ball is moving towards the paddle,
            // otherwise it gets stuck flipping back and forth inside it
            if (speedX < 0 && x > paddle.x) {
                bounceX(); // hit the left paddle
                speedUp();
                aimOff(paddle);
            }
            if (speedX > 0 && x < paddle.x) {
                bounceX(); // hit the right paddle
                speedUp();
                aimOff(paddle);
            }
        }
    }

    void speedUp() {
        speedX *= speedUp;
        speedY *= speedUp;
    }

    // Classic Pong: where the ball hits the paddle decides where it goes.
    // Top edge = goes up, middle = straight, bottom edge = goes down.
    void aimOff(Paddle paddle) {
        double ballMiddle = y + size / 2.0;
        double paddleMiddle = paddle.y + paddle.height / 2.0;

        // -1 = top edge, 0 = middle, 1 = bottom edge
        double hitSpot = (ballMiddle - paddleMiddle) / (paddle.height / 2.0);
        if (hitSpot > 1) {
            hitSpot = 1;
        }
        if (hitSpot < -1) {
            hitSpot = -1;
        }

        speedY = hitSpot * Math.abs(speedX); // at most 45 degrees
    }

    // Getters: let classes outside "objects" read the ball (but not change it)
    public double getX() { return x; }
    public double getY() { return y; }
    public double getSpeedX() { return speedX; }
    public double getSpeedY() { return speedY; }
    public int getSize() { return size; }

    public void bounceX() {
        speedX *= -1;
    }

    public void bounceY() {
        speedY *= -1;
    }
}
