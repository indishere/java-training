// IMPORTS
import java.awt.*;


public class Ball {
    int x = 100;
    int y = 100;
    int size = 20;
    Color color = Color.RED;

    // Constructor
    public Ball(int tempX, int tempY) {
        x = tempX;
        y = tempY;
    }

    public void draw(Graphics graphics) {
        graphics.setColor(color);
        graphics.fillOval(x, y, size, size);
    }
}
