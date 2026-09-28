// DO NOT FUCK WITH THOSE IMPORTS
import java.awt.*;
import java.awt.image.*;
import java.awt.event.*;
import javax.swing.*;

// ====================================== FROGGER GAME ======================================= //
public class frogger extends JFrame implements KeyListener, Runnable {


    // ------------------------------ START OF VARIABLES ------------------------------ //

    // Window
    final String gameTitle = "Frogger";
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
    boolean boostRequested = false;

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
    final int[] vehicleSpeeds = { 420, 300, 240 }; // pixels per second
    final int vehiclePadding = 5;
    final int vehicleHeight = roadHeight - vehiclePadding * 2;
    final int maxVehicles = 10;
    Vehicle[] vehicles = new Vehicle[maxVehicles];
    Vehicle[] leftVehicles = new Vehicle[vehicleNames.length];
    Vehicle[] rightVehicles = new Vehicle[vehicleNames.length];

    // Spawning
    final int spawnDelay = 10000;
    final int roadTwoSpawnOffset = 5000;
    long nextRoadOneSpawn = 0;
    long nextRoadTwoSpawn = 0;

    // Collision
    final int minVisibleAlpha = 128;

    // Debug Mode
    final boolean DEBUG_MODE = true;
    final Color hitboxColor = new Color(255, 0, 0, 120);
    final Color boxOutlineColor = Color.YELLOW;
    boolean debugMode = false;

    // Controls
    boolean movingUp = false;
    boolean movingLeft = false;
    boolean movingDown = false;
    boolean movingRight = false;
    boolean spaceBarDown = false;

    // Game Loop
    final int framesPerSecond = 60;
    final long nanosPerFrame = 1000000000 / framesPerSecond;
    Thread gameThread;
    long lastFrameTime = 0;

    // FPS Counter
    final Font fpsFont = new Font("Arial", Font.BOLD, 20);
    final Color fpsColor = Color.GREEN;
    final int fpsMargin = 10;
    int framesThisSecond = 0;
    int currentFps = 0;
    long fpsTimer = 0;

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
        setIgnoreRepaint(true);
        loadVehicles();
    }

    public void startGame() {
        long currentTime = System.currentTimeMillis();

        createBufferStrategy(2);
        updateRoads();
        placePlayerAtStart();
        nextRoadOneSpawn = currentTime;
        nextRoadTwoSpawn = currentTime + roadTwoSpawnOffset;
        lastFrameTime = System.nanoTime();

        gameThread = new Thread(this);
        gameThread.start();
    }

    public void placePlayerAtStart() {
        Insets borders = getInsets();
        playerX = borders.left + (getWidth() - borders.left - borders.right - playerSize) / 2;
        playerY = getHeight() - borders.bottom - playerSize - playerStartGap;
    }


    // ---------------------------------- GAME LOOP ----------------------------------- //
    @Override
    public void run() {
        while (true) {
            // How much real time passed since the last frame (in seconds)
            long frameStart = System.nanoTime();
            double secondsPassed = (frameStart - lastFrameTime) / 1000000000.0;
            lastFrameTime = frameStart;

            countFps(frameStart);
            updateRoads();
            movePlayer(playerSpeed);

            if (boostRequested) {
                boostPlayer();
                boostRequested = false;
            }

            spawnVehicles();
            moveVehicles(secondsPassed);
            checkCollisions();
            drawGame();

            waitForNextFrame(frameStart);
        }
    }

    public void waitForNextFrame(long frameStart) {
        long nanosLeft = nanosPerFrame - (System.nanoTime() - frameStart);

        // Sleep for most of the time left (sleep is only accurate to about 1-2 ms)
        if (nanosLeft > 2000000) {
            try {
                Thread.sleep((nanosLeft - 2000000) / 1000000);
            } catch (InterruptedException exception) {
                return;
            }
        }

        // Then wait out the last bit exactly
        while (System.nanoTime() - frameStart < nanosPerFrame) {
            Thread.yield();
        }
    }


    // ---------------------------------- MAIN DRAW ----------------------------------- //
    public void drawGame() {
        // Draw on a hidden screen first, then show it all at once so nothing flashes
        BufferStrategy screenBuffer = getBufferStrategy();
        Graphics graphics = screenBuffer.getDrawGraphics();

        drawBackground(graphics);
        drawRoads(graphics);
        drawVehicles(graphics);
        drawPlayer(graphics);

        if (debugMode) {
            drawHitboxes(graphics);
        }

        drawFps(graphics);

        graphics.dispose();
        screenBuffer.show();
        Toolkit.getDefaultToolkit().sync();
    }

    public void drawBackground(Graphics graphics) {
        graphics.setColor(backgroundColor);
        graphics.fillRect(0, 0, getWidth(), getHeight());
    }


    // --------------------------------- FPS COUNTER ---------------------------------- //
    public void countFps(long frameStart) {
        framesThisSecond++;

        // Once a second has passed, save the count and start over
        if (frameStart - fpsTimer >= 1000000000) {
            currentFps = framesThisSecond;
            framesThisSecond = 0;
            fpsTimer = frameStart;
        }
    }

    public void drawFps(Graphics graphics) {
        Insets borders = getInsets();
        int textX = borders.left + fpsMargin;
        int textY = borders.top + fpsMargin + fpsFont.getSize();

        graphics.setFont(fpsFont);
        graphics.setColor(fpsColor);
        graphics.drawString("FPS: " + currentFps + "/" + framesPerSecond, textX, textY);
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
    public void loadVehicles() {
        // Build every vehicle once so spawning later is instant
        for (int type = 0; type < vehicleNames.length; type++) {
            leftVehicles[type] = loadVehicle(vehicleNames[type] + "_left.png");
            rightVehicles[type] = loadVehicle(vehicleNames[type] + "_right.png");
        }
    }

    public Vehicle loadVehicle(String fileName) {
        BufferedImage image = loadVehicleImage(fileName);
        return new Vehicle(image, makeHitboxImage(image));
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
        Vehicle vehicle;

        if (isMovingRight) {
            vehicle = new Vehicle(rightVehicles[type].image, rightVehicles[type].hitbox);
            vehicle.x = roadX - vehicle.image.getWidth();
            vehicle.speed = vehicleSpeeds[type];
        } else {
            vehicle = new Vehicle(leftVehicles[type].image, leftVehicles[type].hitbox);
            vehicle.x = roadX + roadWidth;
            vehicle.speed = -vehicleSpeeds[type];
        }

        vehicle.exactX = vehicle.x;
        vehicle.y = roadY + vehiclePadding;
        addVehicle(vehicle);
    }

    public void addVehicle(Vehicle vehicle) {
        // Put the vehicle in the first empty slot
        for (int index = 0; index < vehicles.length; index++) {
            if (vehicles[index] == null) {
                vehicles[index] = vehicle;
                return;
            }
        }
    }

    public void moveVehicles(double secondsPassed) {
        for (int index = 0; index < vehicles.length; index++) {
            Vehicle vehicle = vehicles[index];

            if (vehicle == null) {
                continue;
            }

            // Move by time (not by frame) and keep the fractions of a pixel
            vehicle.exactX += vehicle.speed * secondsPassed;
            vehicle.x = (int) Math.round(vehicle.exactX);

            boolean isPastRightEdge = vehicle.x > roadX + roadWidth;
            boolean isPastLeftEdge = vehicle.x + vehicle.image.getWidth() < roadX;

            // Free up the slot once the vehicle drives off the screen
            if (isPastRightEdge || isPastLeftEdge) {
                vehicles[index] = null;
            }
        }
    }

    public void drawVehicles(Graphics graphics) {
        for (Vehicle vehicle : vehicles) {
            if (vehicle != null) {
                graphics.drawImage(vehicle.image, vehicle.x, vehicle.y, null);
            }
        }
    }


    // ------------------------------- COLLISION SYSTEM ------------------------------- //
    public void checkCollisions() {
        for (Vehicle vehicle : vehicles) {
            if (vehicle != null && isPlayerHitting(vehicle)) {
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


    // ---------------------------------- DEBUG MODE ---------------------------------- //
    public BufferedImage makeHitboxImage(BufferedImage vehicleImage) {
        int width = vehicleImage.getWidth();
        int height = vehicleImage.getHeight();
        BufferedImage hitbox = new BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB);

        // Paint every visible vehicle pixel in the hitbox color
        for (int imageY = 0; imageY < height; imageY++) {
            for (int imageX = 0; imageX < width; imageX++) {
                Color pixelColor = new Color(vehicleImage.getRGB(imageX, imageY), true);

                if (pixelColor.getAlpha() >= minVisibleAlpha) {
                    hitbox.setRGB(imageX, imageY, hitboxColor.getRGB());
                }
            }
        }

        return hitbox;
    }

    public void drawHitboxes(Graphics graphics) {
        // Vehicles: box for the quick check, red pixels for the real hitbox
        for (Vehicle vehicle : vehicles) {
            if (vehicle != null) {
                graphics.setColor(boxOutlineColor);
                graphics.drawRect(vehicle.x, vehicle.y, vehicle.image.getWidth(), vehicleHeight);
                graphics.drawImage(vehicle.hitbox, vehicle.x, vehicle.y, null);
            }
        }

        // Player: box for the quick check, red circle for the real hitbox
        graphics.setColor(boxOutlineColor);
        graphics.drawRect(playerX, playerY, playerSize, playerSize);
        graphics.setColor(hitboxColor);
        graphics.fillOval(playerX, playerY, playerSize, playerSize);
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

        // --- Shift + D: Debug Mode (only works when DEBUG_MODE is true) ---
        if (DEBUG_MODE && keyCode == KeyEvent.VK_D && event.isShiftDown()) {
            debugMode = !debugMode;
            return;
        }

        // --- Boost (once per press) ---
        if (keyCode == KeyEvent.VK_SPACE && !spaceBarDown) {
            spaceBarDown = true;
            boostRequested = true;
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
        BufferedImage hitbox;
        double exactX = 0;
        int x = 0;
        int y = 0;
        int speed = 0;

        Vehicle(BufferedImage image, BufferedImage hitbox) {
            this.image = image;
            this.hitbox = hitbox;
        }
    }
}

// --------------------------------- THE END OF CODE ---------------------------------- //
