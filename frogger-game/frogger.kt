import java.awt.*
import javax.swing.JFrame
import javax.swing.SwingUtilities

// This is the main window for the Frogger game.
class Frogger : JFrame() {

    // Set up the window when we create a new Frogger game.
    init {
        title = "Frogger"
        setSize(400, 300)
        defaultCloseOperation = JFrame.EXIT_ON_CLOSE
        setLocationRelativeTo(null) // Put the window in the middle of the screen.
    }

    // This method runs whenever Java needs to draw the window.
    override fun paint(graphics: Graphics) {
        super.paint(graphics) // Draw the normal window before drawing our game.

        val centerX = width / 2
        val centerY = height / 2
        val faceSize = 120

        graphics.color = Color.YELLOW
        graphics.fillOval(centerX - faceSize / 2, centerY - faceSize / 2, faceSize, faceSize)

        graphics.color = Color.BLACK
        graphics.drawOval(centerX - faceSize / 2, centerY - faceSize / 2, faceSize, faceSize)

        graphics.fillOval(centerX - 20, centerY - 20, 10, 10)
        graphics.fillOval(centerX + 10, centerY - 20, 10, 10)

        graphics.color = Color.RED
        graphics.drawArc(centerX - 30, centerY - 10, 60, 40, 210, 120)
    }
}

// Kotlin starts running here.
fun main() {
    SwingUtilities.invokeLater {
        val gameWindow = Frogger()
        gameWindow.isVisible = true
    }
}
