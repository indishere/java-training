// IMPORTS
import processing.core.*;


// ====================================== FROGGER GAME ======================================= //
public class frogger extends PApplet {


    // ------------------------------ START OF VARIABLES ------------------------------ //
    // Colors are hex codes: 0xAARRGGBB (AA is how solid it is, FF = fully solid)

    // Window
    final String gameTitle = "Frogger";
    final int windowWidth = 1600;
    final int windowHeight = 900;
    final int framesPerSecond = 60;
    final int backgroundColor = 0xFF000000; // black

    // Player (the frog)
    final int playerSize = 50;
    final int playerRadius = playerSize / 2;
    final int playerSpeed = 5;
    final int playerStartGap = playerSize / 2;
    final int playerColor = 0xFF00FF00; // green
    int playerX = 0;
    int playerY = 0;

    // Jump (water and anything shorter than jumpHeight can't get the frog while it's in the air)
    final int jumpLength = 240; // pixels the frog flies if you hold a direction the whole jump
    final int jumpHeight = 60; // how high the frog gets off the ground
    final int jumpDuration = 500; // milliseconds in the air
    final int jumpCooldown = 1000;
    final int jumpFrames = jumpDuration * framesPerSecond / 1000; // 30 frames in the air
    final int jumpSpeed = jumpLength / jumpFrames; // pixels per frame while in the air
    final int jumpGrowth = playerSize / 5; // how much bigger the frog looks in the air
    boolean isJumping = false;
    int jumpStartTime = -jumpCooldown;

    // Roads
    final int roadHeight = (int) (playerSize * 3 * 1.25);
    final int laneGap = playerSize; // empty space between the roads and the river
    final int roadColor = 0xFF404040; // dark gray
    int roadOneY = 0;
    int roadTwoY = 0;

    // River
    final int riverHeight = roadHeight;
    final int riverColor = 0xFF1E6BD6; // blue
    int riverY = 0;

    // Logs
    final String[] logNames = { "birch_log", "mossy_log", "oak_log", "short_log" };
    final int maxLogHeight = (int) (riverHeight * 0.75);
    final int minLogHeight = maxLogHeight * 3 / 4;
    final int minLogSpeed = 2; // pixels per frame
    final int maxLogSpeed = 4; // pixels per frame
    final int logSpawnDelay = 3000;
    final int logGap = playerSize * 2; // closest a log can get to the log in front of it
    final int maxLogs = 10;
    Log[] logs = new Log[maxLogs];
    PImage[] logImages = new PImage[logNames.length];
    Log lastLog = null;
    int nextLogSpawn = 0;
    int nextLogSpeed = 0;

    // Vehicles
    final String imageFolder = "assets/images/";
    final String[] vehicleNames = { "car", "truck", "bus" };
    final int[] vehicleSpeeds = { 7, 5, 4 }; // pixels per frame
    final int[] vehicleTallness = { 40, 100, 90 }; // how tall each vehicle is off the ground
    final int vehiclePadding = 5;
    final int vehicleHeight = roadHeight - vehiclePadding * 2;
    final int maxVehicles = 10;
    Vehicle[] vehicles = new Vehicle[maxVehicles];
    Vehicle[] leftVehicles = new Vehicle[vehicleNames.length];
    Vehicle[] rightVehicles = new Vehicle[vehicleNames.length];

    // Spawning
    final int spawnDelay = 10000;
    final int roadTwoSpawnOffset = 5000;
    int nextRoadOneSpawn = 0;
    int nextRoadTwoSpawn = 0;

    // Collision
    final int minVisibleAlpha = 128;

    // Debug Mode
    final boolean DEBUG_MODE = true;
    final int hitboxColor = 0x78FF0000; // see-through red
    final int boxOutlineColor = 0xFFFFFF00; // yellow
    boolean debugMode = false;

    // Controls
    boolean movingUp = false;
    boolean movingLeft = false;
    boolean movingDown = false;
    boolean movingRight = false;
    boolean spaceBarDown = false;

    // FPS Counter
    final int fpsTextSize = 20;
    final int fpsColor = 0xFF00FF00; // green
    final int fpsMargin = 10;
    PFont fpsFont;

    // Loading
    boolean imagesLoaded = false;

    // ------------------------------- END OF VARIABLES ------------------------------- //


    // ------------------------------------ SETUP ------------------------------------- //
    public void settings() {
        size(windowWidth, windowHeight, P2D);

        // Load the images while the window is still opening
        thread("loadImages");
    }

    public void setup() {
        surface.setTitle(gameTitle);
        frameRate(framesPerSecond);
        ellipseMode(CORNER);
        fpsFont = createFont("Arial Bold", fpsTextSize);

        placeLanes();
        placePlayerAtStart();

        // Wait in case the images are still loading
        while (!imagesLoaded) {
            delay(10);
        }

        nextRoadOneSpawn = millis();
        nextRoadTwoSpawn = millis() + roadTwoSpawnOffset;
        nextLogSpawn = millis();
        nextLogSpeed = (int) random(minLogSpeed, maxLogSpeed + 1);
    }

    public void loadImages() {
        loadVehicles();
        loadLogs();
        imagesLoaded = true;
    }

    public void placePlayerAtStart() {
        playerX = (width - playerSize) / 2;
        playerY = height - playerSize - playerStartGap;
    }


    // ---------------------------------- GAME LOOP ----------------------------------- //
    public void draw() {
        updateGame();
        drawGame();
    }

    public void updateGame() {
        updateJump();

        if (isJumping) {
            movePlayer(jumpSpeed);
        } else {
            movePlayer(playerSpeed);
        }

        spawnVehicles();
        moveVehicles();
        spawnLogs();
        moveLogs();

        checkCollisions();

        // Water can't get the frog while it's in the air
        if (!isJumping) {
            checkRiver();
        }
    }


    // ---------------------------------- MAIN DRAW ----------------------------------- //
    public void drawGame() {
        background(backgroundColor);
        drawRoads();
        drawRiver();
        drawLogs();
        drawVehicles();
        drawPlayer();

        if (debugMode) {
            drawHitboxes();
        }

        drawFps();
    }


    // --------------------------------- FPS COUNTER ---------------------------------- //
    public void drawFps() {
        textFont(fpsFont);
        textAlign(LEFT, TOP);
        fill(fpsColor);
        text("FPS: " + round(frameRate) + "/" + framesPerSecond, fpsMargin, fpsMargin);
    }


    // --------------------------------- ROAD SYSTEM ---------------------------------- //
    public void placeLanes() {
        // From top to bottom: Road One, gap, River, gap, Road Two
        int totalHeight = roadHeight + laneGap + riverHeight + laneGap + roadHeight;

        roadOneY = (height - totalHeight) / 2;
        riverY = roadOneY + roadHeight + laneGap;
        roadTwoY = riverY + riverHeight + laneGap;
    }

    public void drawRoads() {
        drawRoad(roadOneY);
        drawRoad(roadTwoY);
    }

    public void drawRoad(int roadY) {
        noStroke();
        fill(roadColor);
        rect(0, roadY, width, roadHeight);
    }


    // --------------------------------- RIVER SYSTEM --------------------------------- //
    public void drawRiver() {
        noStroke();
        fill(riverColor);
        rect(0, riverY, width, riverHeight);
    }

    public void loadLogs() {
        for (int type = 0; type < logNames.length; type++) {
            logImages[type] = loadImage(imageFolder + "logs/" + logNames[type] + ".png");
        }
    }

    public void spawnLogs() {
        // Logs float in from the left edge and drift to the right
        if (millis() >= nextLogSpawn && hasRoomForLog(nextLogSpeed)) {
            int type = (int) random(logNames.length);
            int logHeight = (int) random(minLogHeight, maxLogHeight + 1);

            Log log = new Log();
            log.image = resizeSharp(logImages[type], logHeight);
            log.speed = nextLogSpeed;
            log.x = -log.image.width;
            log.y = riverY + (riverHeight - logHeight) / 2; // centered in the river

            addLog(log);
            lastLog = log;
            nextLogSpawn = millis() + logSpawnDelay;
            nextLogSpeed = (int) random(minLogSpeed, maxLogSpeed + 1);
        }
    }

    public boolean hasRoomForLog(int speed) {
        if (lastLog == null || lastLog.x > width) {
            return true;
        }

        // How much closer a new log gets to the last log before the last log floats off the screen
        int framesLeft = (width - lastLog.x) / lastLog.speed;
        int distanceGained = (speed - lastLog.speed) * framesLeft;

        return lastLog.x - distanceGained >= logGap;
    }

    public void addLog(Log log) {
        // Put the log in the first empty slot
        for (int index = 0; index < logs.length; index++) {
            if (logs[index] == null) {
                logs[index] = log;
                return;
            }
        }
    }

    public void moveLogs() {
        for (int index = 0; index < logs.length; index++) {
            Log log = logs[index];

            if (log == null) {
                continue;
            }

            log.x += log.speed;

            // Free up the slot once the log floats off the screen
            if (log.x > width) {
                logs[index] = null;
            }
        }
    }

    public void drawLogs() {
        for (Log log : logs) {
            if (log != null) {
                image(log.image, log.x, log.y);
            }
        }
    }

    public void checkRiver() {
        int playerCenterX = playerX + playerRadius;
        int playerCenterY = playerY + playerRadius;

        boolean isInRiver = playerCenterY >= riverY && playerCenterY < riverY + riverHeight;

        if (!isInRiver) {
            return;
        }

        // Standing on a log: float along with it
        for (Log log : logs) {
            if (log != null && isOnLog(log, playerCenterX, playerCenterY)) {
                playerX += log.speed;
                return;
            }
        }

        // No log under the frog: it fell in the water
        placePlayerAtStart();
    }

    public boolean isOnLog(Log log, int pointX, int pointY) {
        return pointX >= log.x && pointX < log.x + log.image.width
                && pointY >= log.y && pointY < log.y + log.image.height;
    }


    // -------------------------------- VEHICLE SYSTEM -------------------------------- //
    public void loadVehicles() {
        // Build every vehicle once so spawning later is instant
        for (int type = 0; type < vehicleNames.length; type++) {
            String name = vehicleNames[type];
            leftVehicles[type] = loadVehicle(name + "/" + name + "_left.png");
            rightVehicles[type] = loadVehicle(name + "/" + name + "_right.png");
        }

    }

    public Vehicle loadVehicle(String fileName) {
        PImage image = resizeSharp(loadImage(imageFolder + fileName), vehicleHeight);
        return new Vehicle(image, makeHitboxImage(image));
    }

    public PImage resizeSharp(PImage original, int newHeight) {
        int newWidth = original.width * newHeight / original.height;
        PImage resized = createImage(newWidth, newHeight, ARGB);

        original.loadPixels();
        resized.loadPixels();

        // Copy the nearest original pixel into each new pixel so the pixel art stays sharp
        for (int y = 0; y < newHeight; y++) {
            for (int x = 0; x < newWidth; x++) {
                int originalX = x * original.width / newWidth;
                int originalY = y * original.height / newHeight;
                resized.pixels[y * newWidth + x] = original.pixels[originalY * original.width + originalX];
            }
        }

        resized.updatePixels();
        return resized;
    }

    public void spawnVehicles() {
        // Road One (drives left to right)
        if (millis() >= nextRoadOneSpawn) {
            spawnVehicle(roadOneY, true);
            nextRoadOneSpawn += spawnDelay;
        }

        // Road Two (drives right to left)
        if (millis() >= nextRoadTwoSpawn) {
            spawnVehicle(roadTwoY, false);
            nextRoadTwoSpawn += spawnDelay;
        }
    }

    public void spawnVehicle(int roadY, boolean isMovingRight) {
        int type = (int) random(vehicleNames.length);
        Vehicle vehicle;

        if (isMovingRight) {
            vehicle = new Vehicle(rightVehicles[type].image, rightVehicles[type].hitbox);
            vehicle.x = -vehicle.image.width;
            vehicle.speed = vehicleSpeeds[type];
        } else {
            vehicle = new Vehicle(leftVehicles[type].image, leftVehicles[type].hitbox);
            vehicle.x = width;
            vehicle.speed = -vehicleSpeeds[type];
        }

        vehicle.y = roadY + vehiclePadding;
        vehicle.tallness = vehicleTallness[type];
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

    public void moveVehicles() {
        for (int index = 0; index < vehicles.length; index++) {
            Vehicle vehicle = vehicles[index];

            if (vehicle == null) {
                continue;
            }

            vehicle.x += vehicle.speed;

            boolean isPastRightEdge = vehicle.x > width;
            boolean isPastLeftEdge = vehicle.x + vehicle.image.width < 0;

            // Free up the slot once the vehicle drives off the screen
            if (isPastRightEdge || isPastLeftEdge) {
                vehicles[index] = null;
            }
        }
    }

    public void drawVehicles() {
        for (Vehicle vehicle : vehicles) {
            if (vehicle != null) {
                image(vehicle.image, vehicle.x, vehicle.y);
            }
        }
    }


    // ------------------------------- COLLISION SYSTEM ------------------------------- //
    public void checkCollisions() {
        for (Vehicle vehicle : vehicles) {
            if (vehicle == null) {
                continue;
            }

            // The frog can only jump over vehicles that are shorter than its jump
            boolean isJumpingOver = isJumping && jumpHeight > vehicle.tallness;

            if (!isJumpingOver && isPlayerHitting(vehicle)) {
                placePlayerAtStart();
                return;
            }
        }
    }

    public boolean isPlayerHitting(Vehicle vehicle) {
        // Quick check: skip vehicles whose box doesn't touch the player's box
        boolean boxesTouch = playerX < vehicle.x + vehicle.image.width
                && playerX + playerSize > vehicle.x
                && playerY < vehicle.y + vehicle.image.height
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

        boolean isOnImage = imageX >= 0 && imageX < vehicle.image.width
                && imageY >= 0 && imageY < vehicle.image.height;

        if (!isOnImage) {
            return false;
        }

        int pixelColor = vehicle.image.pixels[imageY * vehicle.image.width + imageX];
        return getAlpha(pixelColor) >= minVisibleAlpha;
    }

    public int getAlpha(int pixelColor) {
        // The alpha is the AA part of 0xAARRGGBB, so shift it down and keep only those two digits
        return (pixelColor >> 24) & 0xFF;
    }


    // ---------------------------------- DEBUG MODE ---------------------------------- //
    public PImage makeHitboxImage(PImage vehicleImage) {
        PImage hitbox = createImage(vehicleImage.width, vehicleImage.height, ARGB);
        hitbox.loadPixels();

        // Paint every visible vehicle pixel in the hitbox color
        for (int index = 0; index < vehicleImage.pixels.length; index++) {
            if (getAlpha(vehicleImage.pixels[index]) >= minVisibleAlpha) {
                hitbox.pixels[index] = hitboxColor;
            }
        }

        hitbox.updatePixels();
        return hitbox;
    }

    public void drawHitboxes() {
        // Vehicles: box for the quick check, red pixels for the real hitbox
        for (Vehicle vehicle : vehicles) {
            if (vehicle != null) {
                noFill();
                stroke(boxOutlineColor);
                rect(vehicle.x, vehicle.y, vehicle.image.width, vehicle.image.height);
                image(vehicle.hitbox, vehicle.x, vehicle.y);
            }
        }

        // Logs: the frog is safe while its center is inside one of these boxes
        for (Log log : logs) {
            if (log != null) {
                noFill();
                stroke(boxOutlineColor);
                rect(log.x, log.y, log.image.width, log.image.height);
            }
        }

        // Player: box for the quick check, red circle for the real hitbox
        noFill();
        stroke(boxOutlineColor);
        rect(playerX, playerY, playerSize, playerSize);
        noStroke();
        fill(hitboxColor);
        ellipse(playerX, playerY, playerSize, playerSize);
    }


    // --------------------------------- PLAYER CODE ---------------------------------- //
    public void drawPlayer() {
        noStroke();
        fill(playerColor);

        if (isJumping) {
            // Draw the frog a bit bigger so it looks like it's in the air
            int jumpSize = playerSize + jumpGrowth * 2;
            ellipse(playerX - jumpGrowth, playerY - jumpGrowth, jumpSize, jumpSize);
        } else {
            ellipse(playerX, playerY, playerSize, playerSize);
        }
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
        checkEdges();
    }

    public void jumpPlayer() {
        if (isJumping || millis() - jumpStartTime < jumpCooldown) {
            return;
        }

        isJumping = true;
        jumpStartTime = millis();
    }

    public void updateJump() {
        // Land once the jump time is over
        if (isJumping && millis() - jumpStartTime >= jumpDuration) {
            isJumping = false;
        }
    }


    // ------------------------------------ WALLS ------------------------------------- //
    public void checkEdges() {
        int rightEdge = width - playerSize;
        int bottomEdge = height - playerSize;

        // Left Wall
        if (playerX < 0) {
            playerX = rightEdge;
        }

        // Right Wall
        if (playerX > rightEdge) {
            playerX = 0;
        }

        // Top Wall
        if (playerY < 0) {
            playerY = bottomEdge;
        }

        // Bottom Wall
        if (playerY > bottomEdge) {
            playerY = 0;
        }
    }


    // ------------------------------ KEYBOARD CONTROLS ------------------------------- //
    public void setDirection(boolean isPressed) {
        // --- E / Up Arrow ---
        if (keyCode == 'E' || keyCode == UP) {
            movingUp = isPressed;
        }

        // --- S / Left Arrow ---
        if (keyCode == 'S' || keyCode == LEFT) {
            movingLeft = isPressed;
        }

        // --- D / Down Arrow ---
        if (keyCode == 'D' || keyCode == DOWN) {
            movingDown = isPressed;
        }

        // --- F / Right Arrow ---
        if (keyCode == 'F' || keyCode == RIGHT) {
            movingRight = isPressed;
        }
    }

    public void keyPressed() {
        // --- Shift + D: Debug Mode (only works when DEBUG_MODE is true) ---
        if (DEBUG_MODE && key == 'D') {
            debugMode = !debugMode;
            return;
        }

        // --- Jump (once per press) ---
        if (key == ' ' && !spaceBarDown) {
            spaceBarDown = true;
            jumpPlayer();
        }

        setDirection(true);
    }

    public void keyReleased() {
        if (key == ' ') {
            spaceBarDown = false;
        }

        setDirection(false);
    }


    // -------------------------------- START THE GAME -------------------------------- //
    public static void main(String[] args) {
        // Skip looking for phone-style OpenGL (OpenGL ES) so the window opens faster
        System.setProperty("jogl.disable.opengles", "true");
        PApplet.main("frogger");
    }


    // -------------------------------- VEHICLE CLASS --------------------------------- //
    class Vehicle {
        PImage image;
        PImage hitbox;
        int x = 0;
        int y = 0;
        int speed = 0;
        int tallness = 0;

        Vehicle(PImage image, PImage hitbox) {
            this.image = image;
            this.hitbox = hitbox;
        }
    }


    // ---------------------------------- LOG CLASS ----------------------------------- //
    class Log {
        PImage image;
        int x = 0;
        int y = 0;
        int speed = 0;
    }
}

// --------------------------------- THE END OF CODE ---------------------------------- //
