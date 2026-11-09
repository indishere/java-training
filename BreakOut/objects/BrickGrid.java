package objects;

// IMPORTS
import java.awt.*;
import java.util.ArrayList;
import java.util.List;


// Builds and manages the wall of bricks for one level.
public class BrickGrid {
    private static final int COLUMNS = 10;
    private static final int SIDE_MARGIN = 50;
    private static final int TOP = 70;
    private static final int GAP = 5;
    private static final int HEIGHT = 22;

    private static final Color BRICK_COLOR = Color.BLUE;

    private final List<Brick> bricks = new ArrayList<>();
    private final int rows;

    public BrickGrid(int screenWidth, int level) {
        rows = Math.min(9, 5 + level);

        for (int row = 0; row < rows; row++) {
            int points = (rows - row) * 10;

            for (int column = 0; column < COLUMNS; column++) {
                bricks.add(new Brick(0, 0, 0, 0, BRICK_COLOR, points));
            }
        }

        reflow(screenWidth);
    }

    // Reposition the bricks when the window changes size
    public void reflow(int screenWidth) {
        int usableWidth = screenWidth - SIDE_MARGIN * 2;
        int brickWidth = (usableWidth - GAP * (COLUMNS - 1)) / COLUMNS;

        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < COLUMNS; column++) {
                int x = SIDE_MARGIN + column * (brickWidth + GAP);
                int y = TOP + row * (HEIGHT + GAP);
                Brick brick = bricks.get(row * COLUMNS + column);
                brick.setBounds(x, y, brickWidth, HEIGHT);
            }
        }
    }

    // Returns the brick the ball hit, or null if it did not hit one.
    public Brick collide(Ball ball) {
        for (Brick brick : bricks) {
            if (brick.isAlive() && ball.bounceOff(brick)) {
                brick.destroy();
                return brick;
            }
        }
        return null;
    }

    public void draw(Graphics g) {
        for (Brick brick : bricks) {
            brick.draw(g);
        }
    }

    public boolean isCleared() {
        for (Brick brick : bricks) {
            if (brick.isAlive()) {
                return false;
            }
        }
        return true;
    }
}
