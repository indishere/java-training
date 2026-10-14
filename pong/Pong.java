// IMPORTS
import java.awt.*;
import javax.swing.*;


public class Pong extends JFrame {
    // Global Variables
    Ball ball = new Ball(375, 275);

    public Pong() {
        setSize(800, 600);
    }

    public void paint(Graphics graphics) {
        ball.draw(graphics);
    }


    public static void main(String[] args) {
    Pong pong = new Pong();
    pong.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
    pong.setVisible(true);
    }
}
