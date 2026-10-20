// IMPORTS
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;
import objects.Ball;
import objects.Paddle;


public class Game extends JPanel implements KeyListener {
    // Global Variables
    static final int FPS = 60;
    int width = 800;  
    int height = 600;
    JFrame window;

    Ball ball;
    Paddle leftPaddle;
    Paddle rightPaddle;
    Engine engine; // plays the right paddle

    // What the game is doing right now: "playing", "gameover" or "countdown"
    // Start with the 3, 2, 1 countdown (gives the window time to maximize)
    String state = "countdown";
    int timer = 3 * FPS;  // counts down frames (60 frames = 1 second)
    String message = "3"; // big text in the middle of the screen
    int messageTimer = 0; // how many frames to keep showing "GO!"

    public Game() {
        setPreferredSize(new Dimension(width, height));
        setBackground(Color.BLACK);
        setFocusable(true);
        addKeyListener(this); // send key presses to the methods below
        reset();

        engine = new Engine(this);
        engine.start(); // runs Engine.run() on its own thread
    }

    // Put everything back where it started
    void reset() {
        ball = new Ball(width / 2 - 15, height / 2 - 15); // middle of the screen
        leftPaddle = new Paddle(20, height / 2 - 50);
        rightPaddle = new Paddle(width - 35, height / 2 - 50);
    }

    // KEYBOARD: E / UP = up, D / DOWN = down, F11 = fullscreen, ESC = leave fullscreen
    public void keyPressed(KeyEvent e) {
        int key = e.getKeyCode();
        if (key == KeyEvent.VK_F11) {
            setFullscreen(!window.isUndecorated());
        }
        if (key == KeyEvent.VK_ESCAPE) {
            setFullscreen(false);
        }
        if (key == KeyEvent.VK_E || key == KeyEvent.VK_UP) {
            leftPaddle.movingUp = true;
        }
        if (key == KeyEvent.VK_D || key == KeyEvent.VK_DOWN) {
            leftPaddle.movingDown = true;
        }
    }

    public void keyReleased(KeyEvent e) {
        int key = e.getKeyCode();
        if (key == KeyEvent.VK_E || key == KeyEvent.VK_UP) {
            leftPaddle.movingUp = false;
        }
        if (key == KeyEvent.VK_D || key == KeyEvent.VK_DOWN) {
            leftPaddle.movingDown = false;
        }
    }

    public void keyTyped(KeyEvent e) {
        // not used, but KeyListener needs it
    }

    // Fullscreen = no title bar or borders, and as big as the screen
    void setFullscreen(boolean on) {
        if (window.isUndecorated() == on) {
            return; // already like that
        }
        window.dispose();            // the title bar can only change while the window is hidden
        window.setUndecorated(on);   // undecorated = no title bar
        if (on) {
            window.setExtendedState(JFrame.MAXIMIZED_BOTH);
        } else {
            window.setExtendedState(JFrame.NORMAL);
            window.pack();                     // back to 800x600
            window.setLocationRelativeTo(null); // centred on screen
        }
        window.setVisible(true);
        requestFocusInWindow(); // so the keys still go to the game
    }

    // Called once per frame by the game loop
    public void update() {
        // read the size again in case the window was resized
        if (getWidth() > 0 && getHeight() > 0) {
            boolean sizeChanged = getWidth() != width || getHeight() != height;
            width = getWidth();
            height = getHeight();

            // not playing right now? put everything back in the new middle
            if (sizeChanged && !state.equals("playing")) {
                reset();
            }
        }
        rightPaddle.setX(width - 35); // keep it on the right edge

        if (state.equals("playing")) {
            engine.steer(rightPaddle); // the engine moves the right paddle

            leftPaddle.update(height, ball.getSpeed());
            rightPaddle.update(height, ball.getSpeed());
            ball.update();
            ball.bounceOffWalls(height);
            ball.bounceOffPaddle(leftPaddle);
            ball.bounceOffPaddle(rightPaddle);

            // hide "GO!" after a moment
            if (messageTimer > 0) {
                messageTimer--;
                if (messageTimer == 0) {
                    message = "";
                }
            }

            // ball got past a paddle
            if (ball.hitSideWall(width)) {
                reset();
                state = "gameover";
                message = "GAME OVER";
                timer = 3 * FPS; // 3 seconds
            }
        } else if (state.equals("gameover")) {
            timer--;
            if (timer == 0) {
                state = "countdown";
                message = "3";
                timer = 3 * FPS; // 3 seconds: "3", "2", "1"
            }
        } else if (state.equals("countdown")) {
            timer--;
            if (timer == 2 * FPS) {
                message = "2";
            }
            if (timer == 1 * FPS) {
                message = "1";
            }
            if (timer == 0) {
                state = "playing";
                message = "GO!";
                messageTimer = 1 * FPS; // show "GO!" for 1 second
            }
        }
    }

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g); // clears the last frame
        leftPaddle.draw(g);
        rightPaddle.draw(g);
        ball.draw(g);

        // dark see-through overlay while it says GAME OVER
        if (state.equals("gameover")) {
            g.setColor(new Color(0, 0, 0, 180));
            g.fillRect(0, 0, width, height);
        }

        // big message in the middle of the screen
        if (!message.equals("")) {
            g.setColor(Color.WHITE);
            g.setFont(new Font("Arial", Font.BOLD, 72));
            int textWidth = g.getFontMetrics().stringWidth(message);
            g.drawString(message, (width - textWidth) / 2, height / 2);
        }
    }


    public static void main(String[] args) throws InterruptedException {
        Game game = new Game();

        JFrame window = new JFrame("Pongie");
        game.window = window;
        window.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        window.add(game);
        window.pack();
        window.setResizable(true); // lets you drag the edges and maximize
        window.setExtendedState(JFrame.MAXIMIZED_BOTH); // start maximized
        window.setVisible(true);
        game.requestFocusInWindow(); // so the keys go to the game


        // MAIN GAME LOOP
        while (true) {
            game.update();  // move things
            game.repaint(); // draw things
            Thread.sleep(1000 / FPS);
        }
    }
}
