// IMPORTS
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;

// OBJECTS
import objects.Ball;
import objects.Brick;
import objects.BrickGrid;
import objects.Paddle;


public class Game extends JPanel implements KeyListener {
    // Global settings
    static final int WIDTH = 800;
    static final int HEIGHT = 600;
    static final int FPS = 60;
    static final int MAX_LEVEL = 5;

    enum State {
        READY, PLAYING, PAUSED, LEVEL_CLEAR, GAME_OVER, WON
    }

    State state = State.READY;
    int stateTimer = 0;

    int width = WIDTH;
    int height = HEIGHT;
    int score = 0;
    int lives = 3;
    int level = 1;

    Paddle paddle;
    Ball ball;
    BrickGrid bricks;


    public Game() {
        setPreferredSize(new Dimension(width, height));
        setBackground(Color.BLACK);
        setFocusable(true);
        addKeyListener(this);

        MouseAdapter mouse = new MouseAdapter() {
            @Override
            public void mouseMoved(MouseEvent e) {
                if (state == State.READY || state == State.PLAYING) {
                    paddle.followMouse(e.getX());
                }
            }

            @Override
            public void mouseDragged(MouseEvent e) {
                mouseMoved(e);
            }

            @Override
            public void mousePressed(MouseEvent e) {
                requestFocusInWindow();
            }
        };
        addMouseMotionListener(mouse);
        addMouseListener(mouse);

        paddle = new Paddle(width / 2, height - 42);
        ball = new Ball();
        resetGame();

        new Timer(1000 / FPS, e -> {
            updateGame();
            repaint();
        }).start();
    }

    // Start a brand new game
    void resetGame() {
        score = 0;
        lives = 3;
        level = 1;
        paddle.centerOn(width / 2);
        startLevel(1);
        state = State.READY;
    }

    // Build a fresh wall of bricks and put the ball back on the paddle
    void startLevel(int newLevel) {
        level = newLevel;
        bricks = new BrickGrid(width, level);
        ball.setSpeed(5.6 + level * 0.3);
        ball.reset(paddle.getCenterX(), paddle.getBounds().y - ball.getRadius() - 1);
        paddle.centerOn(width / 2);
        state = State.READY;
    }

    // Called once per frame by the game loop
    void updateGame() {
        updateWindowSize();
        paddle.update(width);

        if (state == State.READY) {
            ball.stickTo(paddle);
        } else if (state == State.PLAYING) {
            ball.update();
            ball.bounceOffWalls(0, width, 0);
            ball.bounceOff(paddle);

            Brick hit = bricks.collide(ball);
            if (hit != null) {
                score += hit.getPoints();
            }

            if (ball.isBelow(height)) {
                loseLife();
            } else if (bricks.isCleared()) {
                state = State.LEVEL_CLEAR;
                stateTimer = 90;
            }
        }

        if (state == State.LEVEL_CLEAR) {
            stateTimer--;
            if (stateTimer <= 0) {
                score += 100 * level;
                if (level >= MAX_LEVEL) {
                    state = State.WON;
                } else {
                    startLevel(level + 1);
                }
            }
        }
    }

    // Keep the game filling the window when it is resized or maximized
    void updateWindowSize() {
        int newWidth = getWidth();
        int newHeight = getHeight();

        if (newWidth > 0 && newHeight > 0 && (newWidth != width || newHeight != height)) {
            width = newWidth;
            height = newHeight;
            paddle.setBottomY(height - 42);
            bricks.reflow(width);

            if (state == State.READY) {
                ball.stickTo(paddle);
            }
        }
    }

    void loseLife() {
        lives--;

        if (lives <= 0) {
            state = State.GAME_OVER;
        } else {
            state = State.READY;
            ball.reset(paddle.getCenterX(), paddle.getBounds().y - ball.getRadius() - 1);
            paddle.centerOn(width / 2);
        }
    }

    // KEYBOARD: arrows or mouse = move, SPACE = launch, P = pause, R = restart
    @Override
    public void keyPressed(KeyEvent e) {
        int key = e.getKeyCode();

        if (key == KeyEvent.VK_LEFT || key == KeyEvent.VK_A) {
            paddle.moveLeft(true);
        }
        if (key == KeyEvent.VK_RIGHT || key == KeyEvent.VK_D) {
            paddle.moveRight(true);
        }

        if (key == KeyEvent.VK_SPACE || key == KeyEvent.VK_ENTER) {
            if (state == State.READY) {
                state = State.PLAYING;
                ball.launch();
            } else if (state == State.GAME_OVER || state == State.WON) {
                resetGame();
            }
        }

        if (key == KeyEvent.VK_P) {
            if (state == State.PLAYING) {
                state = State.PAUSED;
            } else if (state == State.PAUSED) {
                state = State.PLAYING;
            }
        }

        if (key == KeyEvent.VK_R) {
            resetGame();
        }
    }

    @Override
    public void keyReleased(KeyEvent e) {
        int key = e.getKeyCode();
        if (key == KeyEvent.VK_LEFT || key == KeyEvent.VK_A) {
            paddle.moveLeft(false);
        }
        if (key == KeyEvent.VK_RIGHT || key == KeyEvent.VK_D) {
            paddle.moveRight(false);
        }
    }

    @Override
    public void keyTyped(KeyEvent e) {
        // Not used, but KeyListener needs it
    }

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g);

        bricks.draw(g);
        if (state != State.GAME_OVER && state != State.WON) {
            ball.draw(g);
        }
        paddle.draw(g);
        drawMessage(g);
        drawHearts(g);
    }

    // Show three Minecraft-style hardcore hearts in the top-right corner.
    void drawHearts(Graphics g) {
        int startX = width - 20 - (3 * 34);

        for (int i = 0; i < 3; i++) {
            Color fill = i < lives ? Color.RED : new Color(60, 60, 60);
            drawHeart(g, startX + (i * 34), 18, fill, i < lives);
        }
    }

    void drawHeart(Graphics g, int x, int y, Color fill, boolean full) {
        int unit = 2;

        // Each pair is the start and end column of one row segment.
        int[][] rows = {
            {2, 4, 8, 10},
            {1, 5, 7, 11},
            {0, 12},
            {0, 12},
            {0, 12},
            {1, 11},
            {2, 10},
            {3, 9},
            {4, 8},
            {5, 7},
            {6, 6}
        };

        for (int row = 0; row < rows.length; row++) {
            for (int part = 0; part < rows[row].length; part += 2) {
                int start = rows[row][part];
                int end = rows[row][part + 1];

                g.setColor(Color.BLACK);
                g.fillRect(x + start * unit, y + row * unit,
                        (end - start + 1) * unit, unit);

                if (end - start >= 4) {
                    g.setColor(fill);
                    g.fillRect(x + (start + 1) * unit, y + row * unit,
                            (end - start - 1) * unit, unit);
                }
            }
        }

        if (full) {
            g.setColor(new Color(255, 105, 105));
            g.fillRect(x + 3 * unit, y + 2 * unit, 2 * unit, unit);
        }
    }

    void drawMessage(Graphics g) {
        int center = Math.max(360, height / 2 - 30);

        if (state == State.READY) {
            drawCentered(g, "LEVEL " + level, center - 55, new Font("SansSerif", Font.BOLD, 18));
            drawCentered(g, "BREAKOUT", center, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "PRESS SPACE TO LAUNCH", center + 40, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.PAUSED) {
            shadeScreen(g);
            drawCentered(g, "PAUSED", center, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "PRESS P TO CONTINUE", center + 40, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.LEVEL_CLEAR) {
            shadeScreen(g);
            drawCentered(g, "LEVEL " + level + " CLEARED", center, new Font("SansSerif", Font.BOLD, 42));
            drawCentered(g, "+" + (100 * level) + " BONUS", center + 40, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.GAME_OVER) {
            shadeScreen(g);
            drawCentered(g, "GAME OVER", center - 20, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "SCORE " + score, center + 20, new Font("SansSerif", Font.BOLD, 20));
            drawCentered(g, "PRESS SPACE TO PLAY AGAIN", center + 60, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.WON) {
            shadeScreen(g);
            drawCentered(g, "YOU WIN", center - 20, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "SCORE " + score, center + 20, new Font("SansSerif", Font.BOLD, 20));
            drawCentered(g, "PRESS SPACE TO PLAY AGAIN", center + 60, new Font("SansSerif", Font.BOLD, 18));
        }
    }

    void shadeScreen(Graphics g) {
        g.setColor(Color.BLACK);
        g.fillRect(0, 0, width, height);
    }

    void drawCentered(Graphics g, String text, int y, Font font) {
        g.setFont(font);
        g.setColor(Color.WHITE);
        FontMetrics metrics = g.getFontMetrics();
        g.drawString(text, (width - metrics.stringWidth(text)) / 2, y);
    }

    public static void main(String[] args) {
        SwingUtilities.invokeLater(() -> {
            Game game = new Game();
            JFrame window = new JFrame("Breakout");
            window.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
            window.setResizable(true);
            window.setMinimumSize(new Dimension(640, 480));
            window.add(game);
            window.pack();
            window.setLocationRelativeTo(null);
            window.setVisible(true);
            game.requestFocusInWindow();
        });
    }
}
