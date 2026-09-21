import java.awt.*;
import javax.swing.JFrame;
import javax.swing.SwingUtilities;

// This is the main window for the Frogger game.

            /* GLOBAL VARIABLES */
public 

public class frogger extends JFrame {

    // Set up the window when we create a new Frogger game.
    public frogger() {
        setTitle("Frogger");
        setSize(400, 300);
        setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        setLocationRelativeTo(null); // Put the window in the middle of the screen.
    }

    // This method runs whenever Java needs to draw the window.
    @Override
    public void paint(Graphics graphics) {
        super.paint(graphics); // Draw the normal window before drawing our game.

        int centerX = getWidth() / 2;
        int centerY = getHeight() / 2;
        int faceSize = 120;

        graphics.setColor(Color.YELLOW);
        graphics.fillOval(centerX - faceSize / 2, centerY - faceSize / 2, faceSize, faceSize);

        graphics.setColor(Color.BLACK);
        graphics.drawOval(centerX - faceSize / 2, centerY - faceSize / 2, faceSize, faceSize);

        graphics.fillOval(centerX - 20, centerY - 20, 10, 10);
        graphics.fillOval(centerX + 10, centerY - 20, 10, 10);

        graphics.setColor(Color.RED);
        graphics.drawArc(centerX - 30, centerY - 10, 60, 40, 210, 120);
    }

    // Java starts running here.
    public static void main(String[] args) {
        SwingUtilities.invokeLater(() -> {
            frogger gameWindow = new frogger();
            gameWindow.setVisible(true);
        });
    }
}
