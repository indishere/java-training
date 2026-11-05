package objects;

// IMPORTS
import java.awt.*;


// One brick knows its position, colour and score value.
public class Brick {
    private final int x;
    private final int y;
    private final int width;
    private final int height;
    private final Color color;
    private final int points;
    private boolean alive = true;

    public Brick(int x, int y, int width, int height, Color color, int points) {
        this.x = x;
        this.y = y;
        this.width = width;
        this.height = height;
        this.color = color;
        this.points = points;
    }

    public void draw(Graphics g) {
        if (!alive) {
            return;
        }

        g.setColor(color);
        g.fillRect(x, y, width, height);
        g.setColor(Color.BLACK);
        g.drawRect(x, y, width, height);
    }

    public void destroy() {
        alive = false;
    }

    public boolean isAlive() { return alive; }
    public int getPoints() { return points; }

    public Rectangle getBounds() {
        return new Rectangle(x, y, width, height);
    }
}
