package objects;

// IMPORTS
import java.awt.*;


// One brick knows its position, colour and score value.
public class Brick {
    private int x;
    private int y;
    private int width;
    private int height;
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

    public void setBounds(int x, int y, int width, int height) {
        this.x = x;
        this.y = y;
        this.width = width;
        this.height = height;
    }

    public boolean isAlive() { return alive; }
    public int getPoints() { return points; }

    public Rectangle getBounds() {
        return new Rectangle(x, y, width, height);
    }
}
