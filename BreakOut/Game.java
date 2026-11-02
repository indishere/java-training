// IMPORTS
import java.awt.*;
import java.awt.event.*;
import java.awt.image.BufferedImage;
import javax.swing.*;

import objects.Ball;
import objects.Paddle;


public class Game extends JFrame implements KeyListener {
    private static final int SIZE = 800;

    private final Ball ball = new Ball(200, 300);
    private final Paddle paddle = new Paddle(350, 750, SIZE);
    // Draw to an offscreen image first so the screen doesn't flicker
    private final BufferedImage buffer = new BufferedImage(SIZE, SIZE, BufferedImage.TYPE_INT_RGB);

    // Constructor
    public Game() {
        setDefaultCloseOperation(EXIT_ON_CLOSE);
        setSize(SIZE, SIZE);
        addKeyListener(this);
        new Thread(this::gameLoop).start();
    }

    public static void main(String[] args) {
        SwingUtilities.invokeLater(() -> new Game().setVisible(true));
    }

    private void gameLoop() {
        while (true) {
            ball.update();
            paddle.update();
            ball.bounceOff(paddle);
            ball.bounceOffWalls(SIZE, SIZE);
            repaint();
            try {
                Thread.sleep(10);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    @Override
    public void paint(Graphics g) {
        Graphics big = buffer.getGraphics();
        big.setColor(Color.BLACK);
        big.fillRect(0, 0, SIZE, SIZE);
        ball.draw(big);
        paddle.draw(big);
        big.dispose();
        g.drawImage(buffer, 0, 0, null);
    }

    @Override
    public void keyPressed(KeyEvent e) {
        if (e.getKeyCode() == KeyEvent.VK_RIGHT) paddle.moveRight();
        if (e.getKeyCode() == KeyEvent.VK_LEFT) paddle.moveLeft();
    }

    @Override
    public void keyReleased(KeyEvent e) {
        if (e.getKeyCode() == KeyEvent.VK_RIGHT) paddle.stopRight();
        if (e.getKeyCode() == KeyEvent.VK_LEFT) paddle.stopLeft();
    }

    @Override
    public void keyTyped(KeyEvent e) {}
}
