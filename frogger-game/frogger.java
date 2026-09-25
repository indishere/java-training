// DO NOT FUCK WITH THOSE IMPORTS
import java.awt.*;
import java.awt.image.*;
import java.awt.event.*;
import javax.swing.*;

// ====================================== FROGGER GAME ======================================= //
public class frogger extends JFrame implements KeyListener, ActionListener {


    // ------------------------------ START OF VARIABLES ------------------------------ //

    // Window
    final String gameTitle = "Frogger";
    final int frameDelay = 16;
    final int expandDistance = 10;
    final Dimension screenSize = GraphicsEnvironment.getLocalGraphicsEnvironment().getMaximumWindowBounds().getSize();
    final Color backgroundColor = Color.BLACK;

    // Player (the frog)
    final int playerSize = 50;
    final int playerRadius = playerSize / 2;
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
    final int roadHeight = (int) (playerSize * 3 * 1.25);
    final int roadGap = playerSize;
    final Color roadColor = Color.DARK_GRAY;
    int roadX = 0;
    int roadWidth = 0;
    int roadOneY = 0;
    int roadTwoY = 0;

    // Vehicles
    final String imageFolder = "assets/images/";
    final String[] vehicleNames = { "car", "truck", "bus" };
    final int[] vehicleSpeeds = { 7, 5, 4 };
    final int vehiclePadding = 5;
    final int vehicleHeight = roadHeight - vehiclePadding * 2;
    BufferedImage[] vehicleLeftImages = new BufferedImage[vehicleNames.length];
    BufferedImage[] vehicleRightImages = new BufferedImage[vehicleNames.length];
    java.util.ArrayList<Vehicle> vehicles = new java.util.ArrayList<>();

    // Spawning
    final int spawnDelay = 10000;
    final int roadTwoSpawnOffset = 5000;
    long nextRoadOneSpawn = 0;
    long nextRoadTwoSpawn = 0;

    // Collision
    final int minVisibleAlpha = 128;

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
        loadVehicleImages();

        gameTimer = new Timer(frameDelay, this);
    }

    public void startGame() {
        long currentTime = System.currentTimeMillis();

        updateRoads();
        placePlayerAtStart();
        nextRoadOneSpawn = currentTime;
        nextRoadTwoSpawn = currentTime + roadTwoSpawnOffset;
        gameTimer.start();
    }

    public void placePlayerAtStart() {
        Insets borders = getInsets();
        playerX = borders.left + (getWidth() - borders.left - borders.right - playerSize) / 2;
        playerY = getHeight() - borders.bottom - playerSize - playerStartGap;
    }


    // ---------------------------------- GAME LOOP ----------------------------------- //
    @Override
    public void actionPerformed(ActionEvent event) {
        updateRoads();
        movePlayer(playerSpeed);
        spawnVehicles();
        moveVehicles();
        checkCollisions();
        repaint();
    }


    // ---------------------------------- MAIN DRAW ----------------------------------- //
    @Override
    public void paint(Graphics graphics) {
        // Draw on an off-screen image first so the screen doesn't flash
        Image buffer = createImage(getWidth(), getHeight());
        Graphics bufferGraphics = buffer.getGraphics();

        drawBackground(bufferGraphics);
        drawRoads(bufferGraphics);
        drawVehicles(bufferGraphics);
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
        drawRoad(graphics, roadOneY);
        drawRoad(graphics, roadTwoY);
    }

    public void drawRoad(Graphics graphics, int roadY) {
        graphics.setColor(roadColor);
        graphics.fillRect(roadX, roadY, roadWidth, roadHeight);
    }


    // -------------------------------- VEHICLE SYSTEM -------------------------------- //
    public void loadVehicleImages() {
        for (int type = 0; type < vehicleNames.length; type++) {
            vehicleLeftImages[type] = loadVehicleImage(vehicleNames[type] + "_left.png");
            vehicleRightImages[type] = loadVehicleImage(vehicleNames[type] + "_right.png");
        }
    }

    public BufferedImage loadVehicleImage(String fileName) {
        Image original = new ImageIcon(imageFolder + fileName).getImage();
        int vehicleWidth = original.getWidth(null) * vehicleHeight / original.getHeight(null);

        // Resize once here so we can read its pixels later for collision
        BufferedImage resized = new BufferedImage(vehicleWidth, vehicleHeight, BufferedImage.TYPE_INT_ARGB);
        Graphics graphics = resized.getGraphics();
        graphics.drawImage(original, 0, 0, vehicleWidth, vehicleHeight, null);
        graphics.dispose();

        return resized;
    }

    public void spawnVehicles() {
        long currentTime = System.currentTimeMillis();

        // Road One (drives left to right)
        if (currentTime >= nextRoadOneSpawn) {
            spawnVehicle(roadOneY, true);
            nextRoadOneSpawn += spawnDelay;
        }

        // Road Two (drives right to left)
        if (currentTime >= nextRoadTwoSpawn) {
            spawnVehicle(roadTwoY, false);
            nextRoadTwoSpawn += spawnDelay;
        }
    }

    public void spawnVehicle(int roadY, boolean isMovingRight) {
        int type = (int) (Math.random() * vehicleNames.length);
        int vehicleY = roadY + vehiclePadding;

        if (isMovingRight) {
            BufferedImage image = vehicleRightImages[type];
            vehicles.add(new Vehicle(image, roadX - image.getWidth(), vehicleY, vehicleSpeeds[type]));
        } else {
            BufferedImage image = vehicleLeftImages[type];
            vehicles.add(new Vehicle(image, roadX + roadWidth, vehicleY, -vehicleSpeeds[type]));
        }
    }

    public void moveVehicles() {
        // Go backwards so removing a vehicle doesn't skip the next one
        for (int index = vehicles.size() - 1; index >= 0; index--) {
            Vehicle vehicle = vehicles.get(index);
            vehicle.x += vehicle.speed;

            boolean isPastRightEdge = vehicle.x > roadX + roadWidth;
            boolean isPastLeftEdge = vehicle.x + vehicle.image.getWidth() < roadX;

            if (isPastRightEdge || isPastLeftEdge) {
                vehicles.remove(index);
            }
        }
    }

    public void drawVehicles(Graphics graphics) {
        for (Vehicle vehicle : vehicles) {
            graphics.drawImage(vehicle.image, vehicle.x, vehicle.y, null);
        }
    }


    // ------------------------------- COLLISION SYSTEM ------------------------------- //
    public void checkCollisions() {
        for (Vehicle vehicle : vehicles) {
            if (isPlayerHitting(vehicle)) {
                placePlayerAtStart();
                return;
            }
        }
    }

    public boolean isPlayerHitting(Vehicle vehicle) {
        // Quick check: skip vehicles whose box doesn't touch the player's box
        boolean boxesTouch = playerX < vehicle.x + vehicle.image.getWidth()
                && playerX + playerSize > vehicle.x
                && playerY < vehicle.y + vehicleHeight
                && playerY + playerSize > vehicle.y;

        if (!boxesTouch) {
            return false;
        }

        // Pixel check: look for a pixel that is inside the frog AND on a visible part of the vehicle
        for (int pixelY = playerY; pixelY < playerY + playerSize; pixelY++) {
            for (int pixelX = playerX; pixelX < playerX + playerSize; pixelX++) {
                if (isInsidePlayer(pixelX, pixelY) && isVisiblePixel(vehicle, pixelX, pixelY)) {
                    return true;
                }
            }
        }

        return false;
    }

    public boolean isInsidePlayer(int pixelX, int pixelY) {
        int distanceX = pixelX - (playerX + playerRadius);
        int distanceY = pixelY - (playerY + playerRadius);

        return distanceX * distanceX + distanceY * distanceY <= playerRadius * playerRadius;
    }

    public boolean isVisiblePixel(Vehicle vehicle, int pixelX, int pixelY) {
        int imageX = pixelX - vehicle.x;
        int imageY = pixelY - vehicle.y;

        boolean isOnImage = imageX >= 0 && imageX < vehicle.image.getWidth()
                && imageY >= 0 && imageY < vehicle.image.getHeight();

        if (!isOnImage) {
            return false;
        }

        Color pixelColor = new Color(vehicle.image.getRGB(imageX, imageY), true);
        return pixelColor.getAlpha() >= minVisibleAlpha;
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
        gameWindow.startGame();
    }


    // -------------------------------- VEHICLE CLASS --------------------------------- //
    class Vehicle {
        BufferedImage image;
        int x;
        int y;
        int speed;

        Vehicle(BufferedImage image, int x, int y, int speed) {
            this.image = image;
            this.x = x;
            this.y = y;
            this.speed = speed;
        }
    }
}

// --------------------------------- THE END OF CODE ---------------------------------- //
