package objects;

import java.awt.*;
import java.util.ArrayList;
import java.util.List;


// Makes the brick wall for each level.
public class BrickGrid {
    private static final int COLUMNS = 10;
    private static final int ROWS = 5;
    private static final int SIDE_MARGIN = 50;
    private static final int TOP = 70;
    private static final int GAP = 5;
    private static final int HEIGHT = 22;
    private static final Color BRICK_COLOR = Color.BLUE;

    // 1 = brick, 0 = empty space. Each level is 5 rows of 10 columns.
    private static final String[][] LEVELS = {
        {
            "1111111111",
            "1111111111",
            "1111111111",
            "1111111111",
            "1111111111"
        },
        {
            "1010101010",
            "0101010101",
            "1010101010",
            "0101010101",
            "1010101010"
        },
        {
            "0000110000",
            "0001111000",
            "0011111100",
            "0111111110",
            "1111111111"
        },
        {
            "1111111111",
            "1000000001",
            "1001111001",
            "1000000001",
            "1111111111"
        },
        {
            "0000110000",
            "0001111000",
            "0011111100",
            "0001111000",
            "0000110000"
        }
    };

    private final List<Brick> bricks = new ArrayList<>();

    public BrickGrid(int screenWidth, int level) {
        String[] layout = LEVELS[level - 1];

        for (int row = 0; row < ROWS; row++) {
            for (int column = 0; column < COLUMNS; column++) {
                Brick brick = new Brick(0, 0, 0, 0, BRICK_COLOR, (ROWS - row) * 10);

                if (layout[row].charAt(column) == '0') {
                    brick.destroy();
                }

                bricks.add(brick);
            }
        }

        reflow(screenWidth);
    }

    // Move and resize the bricks when the window changes size.
    public void reflow(int screenWidth) {
        int usableWidth = screenWidth - SIDE_MARGIN * 2;
        int brickWidth = (usableWidth - GAP * (COLUMNS - 1)) / COLUMNS;

        for (int row = 0; row < ROWS; row++) {
            for (int column = 0; column < COLUMNS; column++) {
                int x = SIDE_MARGIN + column * (brickWidth + GAP);
                int y = TOP + row * (HEIGHT + GAP);
                bricks.get(row * COLUMNS + column).setBounds(x, y, brickWidth, HEIGHT);
            }
        }
    }

    // Returns the brick that was hit, or null if the ball missed.
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
