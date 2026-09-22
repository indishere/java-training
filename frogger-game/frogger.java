// DO NOT FUCK WITH THE IMPORTS
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;


// Main window for the Frogger game.
public class frogger extends JFrame implements KeyListener {
    int frogX = 200;
    int frogY = 200;
    int frogSize = 50;
    int frogSpeed = 5;
    int expandDistance = 10;

    boolean movingUp = false;
    boolean movingLeft = false;
    boolean movingDown = false;
    boolean movingRight = false;

    Dimension screenSize = Toolkit.getDefaultToolkit().getScreenSize();
    Timer timer;

    public frogger() {
        setTitle("Frogger");
        setSize(400, 300);
        setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        setLocationRelativeTo(null);
        getContentPane().setBackground(Color.BLACK);
        addKeyListener(this);
        setFocusable(true);

        timer = new Timer(16, new ActionListener() {
            public void actionPerformed(ActionEvent event) {
                moveFrog();
                repaint();
            }
        });
        timer.start();
    }

    @Override
    public void paint(Graphics graphics) {
        super.paint(graphics);
        graphics.setColor(Color.GREEN);
        graphics.fillOval(frogX, frogY, frogSize, frogSize);
    }

    public void moveFrog() {
        if (movingUp) {
            frogY = frogY - frogSpeed;
        }
        if (movingLeft) {
            frogX = frogX - frogSpeed;
        }
        if (movingDown) {
            frogY = frogY + frogSpeed;
        }
        if (movingRight) {
            frogX = frogX + frogSpeed;
        }

        checkEdges();
    }

    public void checkEdges() {
        Insets borders = getInsets();
        int leftEdge = borders.left;
        int rightEdge = getWidth() - borders.right - frogSize;
        int topEdge = borders.top;
        int bottomEdge = getHeight() - borders.bottom - frogSize;

        // Start growing before the frog reaches each edge.
        if (movingLeft && frogX <= leftEdge + expandDistance) {
            if (getWidth() < screenSize.width) {
                growLeft();
            } else if (frogX < leftEdge) {
                frogX = rightEdge;
            }
        }

        if (movingRight && frogX >= rightEdge - expandDistance) {
            if (getWidth() < screenSize.width) {
                growRight();
            } else if (frogX > rightEdge) {
                frogX = leftEdge;
            }
        }

        if (movingUp && frogY <= topEdge + expandDistance) {
            if (getHeight() < screenSize.height) {
                growUp();
            } else if (frogY < topEdge) {
                frogY = bottomEdge;
            }
        }

        if (movingDown && frogY >= bottomEdge - expandDistance) {
            if (getHeight() < screenSize.height) {
                growDown();
            } else if (frogY > bottomEdge) {
                frogY = topEdge;
            }
        }
    }

    public void growLeft() {
        if (getX() > 0) {
            setBounds(getX() - frogSpeed, getY(), getWidth() + frogSpeed, getHeight());
            frogX = frogX + frogSpeed;
        } else {
            setSize(getWidth() + frogSpeed, getHeight());
            int leftEdge = getInsets().left + expandDistance;
            if (frogX < leftEdge) {
                frogX = leftEdge;
            }
        }
    }

    public void growRight() {
        if (getX() + getWidth() < screenSize.width) {
            setSize(getWidth() + frogSpeed, getHeight());
        } else {
            setBounds(getX() - frogSpeed, getY(), getWidth() + frogSpeed, getHeight());
        }
    }

    public void growUp() {
        if (getY() > 0) {
            setBounds(getX(), getY() - frogSpeed, getWidth(), getHeight() + frogSpeed);
            frogY = frogY + frogSpeed;
        } else {
            setSize(getWidth(), getHeight() + frogSpeed);
            int topEdge = getInsets().top + expandDistance;
            if (frogY < topEdge) {
                frogY = topEdge;
            }
        }
    }

    public void growDown() {
        if (getY() + getHeight() < screenSize.height) {
            setSize(getWidth(), getHeight() + frogSpeed);
        } else {
            setBounds(getX(), getY() - frogSpeed, getWidth(), getHeight() + frogSpeed);
        }
    }

    @Override
    public void keyPressed(KeyEvent event) {
        if (event.getKeyCode() == KeyEvent.VK_E) {
            movingUp = true;
        }
        if (event.getKeyCode() == KeyEvent.VK_S) {
            movingLeft = true;
        }
        if (event.getKeyCode() == KeyEvent.VK_D) {
            movingDown = true;
        }
        if (event.getKeyCode() == KeyEvent.VK_F) {
            movingRight = true;
        }
    }

    @Override
    public void keyReleased(KeyEvent event) {
        if (event.getKeyCode() == KeyEvent.VK_E) {
            movingUp = false;
        }
        if (event.getKeyCode() == KeyEvent.VK_S) {
            movingLeft = false;
        }
        if (event.getKeyCode() == KeyEvent.VK_D) {
            movingDown = false;
        }
        if (event.getKeyCode() == KeyEvent.VK_F) {
            movingRight = false;
        }
    }

    @Override
    public void keyTyped(KeyEvent event) {
    }

    public static void main(String[] args) {
        frogger gameWindow = new frogger();
        gameWindow.setVisible(true);
    }
}
