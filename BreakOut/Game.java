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
    static final int HUD_HEIGHT = 50;
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
        paddle.update(width);

        if (state == State.READY) {
            ball.stickTo(paddle);
        } else if (state == State.PLAYING) {
            ball.update();
            ball.bounceOffWalls(0, width, HUD_HEIGHT);
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
        drawHud(g);
        drawMessage(g);
    }

    void drawHud(Graphics g) {
        g.setColor(Color.WHITE);
        g.setFont(new Font("SansSerif", Font.BOLD, 18));
        g.drawString("SCORE " + score, 20, 32);
        drawCentered(g, "LEVEL " + level, 32, g.getFont());
        drawRight(g, "LIVES " + lives, width - 20, 32, g.getFont());

        g.drawLine(0, HUD_HEIGHT, width, HUD_HEIGHT);

        g.setFont(new Font("SansSerif", Font.PLAIN, 12));
        drawCentered(g, "ARROWS/MOUSE: MOVE    SPACE: LAUNCH    P: PAUSE    R: RESTART",
                height - 12, g.getFont());
    }

    void drawMessage(Graphics g) {
        if (state == State.READY) {
            drawCentered(g, "BREAKOUT", 380, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "PRESS SPACE TO LAUNCH", 420, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.PAUSED) {
            shadeScreen(g);
            drawCentered(g, "PAUSED", 290, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "PRESS P TO CONTINUE", 330, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.LEVEL_CLEAR) {
            shadeScreen(g);
            drawCentered(g, "LEVEL CLEARED", 290, new Font("SansSerif", Font.BOLD, 42));
        } else if (state == State.GAME_OVER) {
            shadeScreen(g);
            drawCentered(g, "GAME OVER", 270, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "SCORE " + score, 310, new Font("SansSerif", Font.BOLD, 20));
            drawCentered(g, "PRESS SPACE TO PLAY AGAIN", 350, new Font("SansSerif", Font.BOLD, 18));
        } else if (state == State.WON) {
            shadeScreen(g);
            drawCentered(g, "YOU WIN", 270, new Font("SansSerif", Font.BOLD, 48));
            drawCentered(g, "SCORE " + score, 310, new Font("SansSerif", Font.BOLD, 20));
            drawCentered(g, "PRESS SPACE TO PLAY AGAIN", 350, new Font("SansSerif", Font.BOLD, 18));
        }
    }

    void shadeScreen(Graphics g) {
        g.setColor(Color.BLACK);
        g.fillRect(0, HUD_HEIGHT + 1, width, height - HUD_HEIGHT - 1);
    }

    void drawCentered(Graphics g, String text, int y, Font font) {
        g.setFont(font);
        g.setColor(Color.WHITE);
        FontMetrics metrics = g.getFontMetrics();
        g.drawString(text, (width - metrics.stringWidth(text)) / 2, y);
    }

    void drawRight(Graphics g, String text, int rightX, int y, Font font) {
        g.setFont(font);
        g.setColor(Color.WHITE);
        g.drawString(text, rightX - g.getFontMetrics().stringWidth(text), y);
    }

    public static void main(String[] args) {
        SwingUtilities.invokeLater(() -> {
            Game game = new Game();
            JFrame window = new JFrame("Breakout");
            window.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
            window.setResizable(false);
            window.add(game);
            window.pack();
            window.setLocationRelativeTo(null);
            window.setVisible(true);
            game.requestFocusInWindow();
        });
    }
}
