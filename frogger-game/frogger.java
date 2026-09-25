// DO NOT FUCK WITH THOSE IMPORTS
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;

// ====================================== FROGGER GAME ======================================= //

public class frogger extends JFrame implements KeyListener {

    // ------------------------------ START OF VARIABLES ------------------------------ //

    // Window
    final String gameTitle = "Frogger";
    final int frameDelay = 16;
    final int expandDistance = 10;
    final Dimension screenSize = GraphicsEnvironment.getLocalGraphicsEnvironment().getMaximumWindowBounds().getSize();
    final Color backgroundColor = Color.BLACK;

    // Player (the frog)
    final int playerSize = 50;
    final int playerSpeed = 5;
    final int playerStartGap = playerSize / 2;
    final Color playerColor = Color.GREEN;
    int playerX = 0;
    int playerY = 0;

    // Boost
    final int boostSpeed = 40;
    final int boostCooldown = 500;
    long lastBoostTime = 0;

    // Roads
    final int roadHeight = playerSize * 3;
    final int roadGap = playerSize;
    final Color roadColor = Color.DARK_GRAY;
    int roadX = 0;
    int roadWidth = 0;
    int roadOneY = 0;
    int roadTwoY = 0;

    // Road Lines
    final int dashLength = 30;
    final int dashGap = 30;
    final int dashThickness = 4;
    final Color dashColor = Color.WHITE;

    // Controls
    boolean movingUp = false;
    boolean movingLeft = false;
    boolean movingDown = false;
    boolean movingRight = false;
    boolean spaceBarDown = false;

    // Game Loop
    Timer gameTimer;

    // ------------------------------- END OF VARIABLES ------------------------------- //


    // ------------------------------------ SETUP ------------------------------------- //
    public frogger() {
        setTitle(gameTitle);
        setSize(screenSize.width, screenSize.height);
        setLocation(0, 0);
        setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        getContentPane().setBackground(backgroundColor);
        setFocusable(true);
        addKeyListener(this);

        gameTimer = new Timer(frameDelay, event -> {
            movePlayer(playerSpeed);
            repaint();
        });
        gameTimer.start();
    }

    public void placePlayerAtStart() {
        Insets borders = getInsets();
        playerX = borders.left + (getWidth() - borders.left - borders.right - playerSize) / 2;
        playerY = getHeight() - borders.bottom - playerSize - playerStartGap;
    }


    // ---------------------------------- MAIN DRAW ----------------------------------- //
    @Override
    public void paint(Graphics graphics) {
        
        // Draw on an off-screen image first so the screen doesn't flash
        Image buffer = createImage(getWidth(), getHeight());
        Graphics bufferGraphics = buffer.getGraphics();

        drawBackground(bufferGraphics);
        drawRoads(bufferGraphics);
        drawPlayer(bufferGraphics);

        graphics.drawImage(buffer, 0, 0, null);
        bufferGraphics.dispose();
    }

    public void drawBackground(Graphics graphics) {
        graphics.setColor(backgroundColor);
        graphics.fillRect(0, 0, getWidth(), getHeight());
    }


    // --------------------------------- ROAD SYSTEM ---------------------------------- //
    public void updateRoads() {
        Insets borders = getInsets();
        int centerY = borders.top + (getHeight() - borders.top - borders.bottom) / 2;

        roadX = borders.left;
        roadWidth = getWidth() - borders.left - borders.right;
        roadOneY = centerY - roadGap / 2 - roadHeight;
        roadTwoY = centerY + roadGap / 2;
    }

    public void drawRoads(Graphics graphics) {
        updateRoads();
        drawRoad(graphics, roadOneY);
        drawRoad(graphics, roadTwoY);
    }

    public void drawRoad(Graphics graphics, int roadY) {
        // Road
        graphics.setColor(roadColor);
        graphics.fillRect(roadX, roadY, roadWidth, roadHeight);

        // Dashed line in the middle
        graphics.setColor(dashColor);
        int dashY = roadY + (roadHeight - dashThickness) / 2;
        for (int dashX = roadX; dashX < roadX + roadWidth; dashX += dashLength + dashGap) {
            graphics.fillRect(dashX, dashY, dashLength, dashThickness);
        }
    }


    // --------------------------------- PLAYER CODE ---------------------------------- //
    public void drawPlayer(Graphics graphics) {
        graphics.setColor(playerColor);
        graphics.fillOval(playerX, playerY, playerSize, playerSize);
    }

    public void movePlayer(int distance) {
        if (movingUp) {
            playerY -= distance;
        }
        if (movingLeft) {
            playerX -= distance;
        }
        if (movingDown) {
            playerY += distance;
        }
        if (movingRight) {
            playerX += distance;
        }
        checkEdges(distance);
    }

    public void boostPlayer() {
        long currentTime = System.currentTimeMillis();
        boolean isMoving = movingUp || movingLeft || movingDown || movingRight;

        if (currentTime - lastBoostTime < boostCooldown || !isMoving) {
            return;
        }

        movePlayer(boostSpeed);
        lastBoostTime = currentTime;
        repaint();
    }


    // ---------------------------- WALLS + WINDOW GROWING ---------------------------- //
    public void checkEdges(int distance) {
        Insets borders = getInsets();
        int leftEdge = borders.left;
        int rightEdge = getWidth() - borders.right - playerSize;
        int topEdge = borders.top;
        int bottomEdge = getHeight() - borders.bottom - playerSize;
        boolean canGrowWider = getWidth() < screenSize.width;
        boolean canGrowTaller = getHeight() < screenSize.height;

        // Left Wall
        if (movingLeft && playerX <= leftEdge + expandDistance) {
            if (canGrowWider) {
                growLeft(distance);
            } else if (playerX < leftEdge) {
                playerX = rightEdge;
            }
        }

        // Right Wall
        if (movingRight && playerX >= rightEdge - expandDistance) {
            if (canGrowWider) {
                growRight(distance);
            } else if (playerX > rightEdge) {
                playerX = leftEdge;
            }
        }

        // Top Wall
        if (movingUp && playerY <= topEdge + expandDistance) {
            if (canGrowTaller) {
                growUp(distance);
            } else if (playerY < topEdge) {
                playerY = bottomEdge;
            }
        }

        // Bottom Wall
        if (movingDown && playerY >= bottomEdge - expandDistance) {
            if (canGrowTaller) {
                growDown(distance);
            } else if (playerY > bottomEdge) {
                playerY = topEdge;
            }
        }
    }

    public void growLeft(int distance) {
        if (getX() > 0) {
            setBounds(getX() - distance, getY(), getWidth() + distance, getHeight());
            playerX += distance;
        } else {
            setSize(getWidth() + distance, getHeight());
            playerX = Math.max(playerX, getInsets().left + expandDistance);
        }
    }

    public void growRight(int distance) {
        if (getX() + getWidth() < screenSize.width) {
            setSize(getWidth() + distance, getHeight());
        } else {
            setBounds(getX() - distance, getY(), getWidth() + distance, getHeight());
        }
    }

    public void growUp(int distance) {
        if (getY() > 0) {
            setBounds(getX(), getY() - distance, getWidth(), getHeight() + distance);
            playerY += distance;
        } else {
            setSize(getWidth(), getHeight() + distance);
            playerY = Math.max(playerY, getInsets().top + expandDistance);
        }
    }

    public void growDown(int distance) {
        if (getY() + getHeight() < screenSize.height) {
            setSize(getWidth(), getHeight() + distance);
        } else {
            setBounds(getX(), getY() - distance, getWidth(), getHeight() + distance);
        }
    }


    // ------------------------------ KEYBOARD CONTROLS ------------------------------- //
    public void setDirection(int keyCode, boolean isPressed) {
        // --- E / Up Arrow ---
        if (keyCode == KeyEvent.VK_E || keyCode == KeyEvent.VK_UP) {
            movingUp = isPressed;
        }

        // --- S / Left Arrow ---
        if (keyCode == KeyEvent.VK_S || keyCode == KeyEvent.VK_LEFT) {
            movingLeft = isPressed;
        }

        // --- D / Down Arrow ---
        if (keyCode == KeyEvent.VK_D || keyCode == KeyEvent.VK_DOWN) {
            movingDown = isPressed;
        }

        // --- F / Right Arrow ---
        if (keyCode == KeyEvent.VK_F || keyCode == KeyEvent.VK_RIGHT) {
            movingRight = isPressed;
        }
    }

    @Override
    public void keyPressed(KeyEvent event) {
        int keyCode = event.getKeyCode();

        // --- Boost (once per press) ---
        if (keyCode == KeyEvent.VK_SPACE && !spaceBarDown) {
            spaceBarDown = true;
            boostPlayer();
        }

        setDirection(keyCode, true);
    }

    @Override
    public void keyReleased(KeyEvent event) {
        int keyCode = event.getKeyCode();

        if (keyCode == KeyEvent.VK_SPACE) {
            spaceBarDown = false;
        }

        setDirection(keyCode, false);
    }

    @Override
    public void keyTyped(KeyEvent event) {
    }


    // -------------------------------- START THE GAME -------------------------------- //
    public static void main(String[] args) {
        frogger gameWindow = new frogger();
        gameWindow.setVisible(true);
        gameWindow.placePlayerAtStart();
    }
}

// --------------------------------- THE END OF CODE ---------------------------------- //
