// IMPORTS
import java.awt.*;
import javax.swing.*;
import objects.Ball;


public class Game extends JPanel {
    // Global Variables
    static final int FPS = 60;
    Ball ball = new Ball(375, 275);

    public Game() {
        setPreferredSize(new Dimension(800, 600));
        setBackground(Color.BLACK);
    }

    // Called once per frame by the game loop
    public void update() {
        ball.update();
    }

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g); // clears the last frame
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


        // MAIN GAME LOOP
        while (true) {
            game.update();  // move things
            game.repaint(); // draw things
            Thread.sleep(1000 / FPS);
        }
    }
}
