// IMPORTS
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;
import objects.Ball;
import objects.Paddle;


public class Game extends JPanel implements KeyListener {
    // Global Variables
    static final int FPS = 60;
    static final int WIDTH = 800;
    static final int HEIGHT = 600;

    Ball ball = new Ball(WIDTH / 2 - 10, HEIGHT / 2 - 10); // middle of the screen
    Paddle leftPaddle = new Paddle(20, HEIGHT / 2 - 50);
    Paddle rightPaddle = new Paddle(WIDTH - 35, HEIGHT / 2 - 50);

    public Game() {
        setPreferredSize(new Dimension(WIDTH, HEIGHT));
        setBackground(Color.BLACK);
        setFocusable(true);
        addKeyListener(this); // send key presses to the methods below
    }

    // KEYBOARD: E / UP = up, D / DOWN = down
    public void keyPressed(KeyEvent e) {
        int key = e.getKeyCode();
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

    // Called once per frame by the game loop
    public void update() {
        leftPaddle.update(HEIGHT);
        rightPaddle.update(HEIGHT);
        ball.update();
        ball.bounceOffWalls(WIDTH, HEIGHT);
    }

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g); // clears the last frame
        leftPaddle.draw(g);
        rightPaddle.draw(g);
        ball.draw(g);
    }


    public static void main(String[] args) throws InterruptedException {
        Game game = new Game();

        JFrame window = new JFrame("Pongie");
        window.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        window.add(game);
        window.pack();
        window.setResizable(false);
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
