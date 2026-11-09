package objects;

// IMPORTS
import java.awt.*;


// The ball moves itself and bounces off walls, the paddle and bricks.
public class Ball {
    private double x;       // middle of the ball
    private double y;
    private double speedX;
    private double speedY;
    private double speed = 6;
    private final int radius = 16;


    public Ball() {
        reset(0, 0);
    }

    // Put the ball on the paddle without moving it
    public void reset(double startX, double startY) {
        x = startX;
        y = startY;
        speedX = 0;
        speedY = 0;
    }

    // Send the ball up and to a random side
    public void launch() {
        double angle = Math.toRadians(35);
        double direction = Math.random() < 0.5 ? -1 : 1;
        speedX = direction * speed * Math.sin(angle);
        speedY = -speed * Math.cos(angle);
    }

    // Called once per frame by the game loop
    public void update() {
        x += speedX;
        y += speedY;
    }

    // Bounce off the left, right and top walls
    public void bounceOffWalls(int left, int right, int top) {
        if (x - radius < left && speedX < 0) {
            x = left + radius;
            speedX = -speedX;
        }
        if (x + radius > right && speedX > 0) {
            x = right - radius;
            speedX = -speedX;
        }
        if (y - radius < top && speedY < 0) {
            y = top + radius;
            speedY = -speedY;
        }
    }

    // Bounce off the paddle. The hit position controls the angle.
    public boolean bounceOff(Paddle paddle) {
        if (speedY <= 0 || !getBounds().intersects(paddle.getBounds())) {
            return false;
        }

        double offset = (x - paddle.getCenterX()) / (paddle.getWidth() / 2.0);
        offset = Math.max(-1, Math.min(1, offset));

        double angle = Math.toRadians(60) * offset;
        speedX = Math.sin(angle) * speed;
        speedY = -Math.cos(angle) * speed;
        y = paddle.getBounds().y - radius - 1;
        return true;
    }

    // Bounce off one brick. Returns false if the ball is not touching it.
    public boolean bounceOff(Brick brick) {
        Rectangle b = brick.getBounds();
        if (!getBounds().intersects(b)) {
            return false;
        }

        double overlapLeft = (x + radius) - b.x;
        double overlapRight = (b.x + b.width) - (x - radius);
        double overlapTop = (y + radius) - b.y;
        double overlapBottom = (b.y + b.height) - (y - radius);
        double horizontal = Math.min(overlapLeft, overlapRight);
        double vertical = Math.min(overlapTop, overlapBottom);

        if (horizontal < vertical) {
            if (x < b.getCenterX()) {
                x = b.x - radius;
                speedX = -Math.abs(speedX);
            } else {
                x = b.x + b.width + radius;
                speedX = Math.abs(speedX);
            }
        } else {
            if (y < b.getCenterY()) {
                y = b.y - radius;
                speedY = -Math.abs(speedY);
            } else {
                y = b.y + b.height + radius;
                speedY = Math.abs(speedY);
            }
        }
        return true;
    }

    public void draw(Graphics g) {
        g.setColor(Color.WHITE);
        g.fillOval((int) x - radius, (int) y - radius, radius * 2, radius * 2);
    }

    public void stickTo(Paddle paddle) {
        x = paddle.getCenterX();
        y = paddle.getBounds().y - radius - 1;
    }

    public void setSpeed(double newSpeed) {
        speed = newSpeed;
    }

    public Rectangle getBounds() {
        return new Rectangle((int) x - radius, (int) y - radius, radius * 2, radius * 2);
    }

    public boolean isBelow(int height) {
        return y - radius > height;
    }

    public int getRadius() { return radius; }
}
