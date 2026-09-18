import java.awt.Graphics;
import javax.swing.JFrame;
import javax.swing.SwingUtilities;

// This is the main window for the Frogger game.
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

        // Draw a rectangle: x, y, width, height.
        graphics.fillRect(100, 100, 50, 75);
    }

    // Java starts running here.
    public static void main(String[] args) {
        SwingUtilities.invokeLater(() -> {
            frogger gameWindow = new frogger();
            gameWindow.setVisible(true);
        });
    }
}
