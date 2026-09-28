// DO NOT FUCK WITH THOSE IMPORTS
import processing.core.*;


// ====================================== FROGGER GAME ======================================= //
public class frogger extends PApplet {


    // ------------------------------ START OF VARIABLES ------------------------------ //

    // Window
    final String gameTitle = "Frogger";
    final int windowWidth = 1600;
    final int windowHeight = 900;
    final int framesPerSecond = 60;
    final int backgroundColor = color(0);

    // Player (the frog)
    final int playerSize = 50;
    final int playerRadius = playerSize / 2;
    final int playerSpeed = 5;
    final int playerStartGap = playerSize / 2;
    final int playerColor = color(0, 255, 0);
    int playerX = 0;
    int playerY = 0;

    // Boost
    final int boostSpeed = 40;
    final int boostCooldown = 500;
    int lastBoostTime = 0;

    // Roads
    final int roadHeight = (int) (playerSize * 3 * 1.25);
    final int roadGap = playerSize;
    final int roadColor = color(64);
    int roadOneY = 0;
    int roadTwoY = 0;

    // Vehicles
    final String imageFolder = "assets/images/";
    final String[] vehicleNames = { "car", "truck", "bus" };
    final int[] vehicleSpeeds = { 7, 5, 4 }; // pixels per frame
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
    final boolean DEBUG_MODE = false;
    final int hitboxColor = color(255, 0, 0, 120);
    final int boxOutlineColor = color(255, 255, 0);
    boolean debugMode = false;

    // Controls
    boolean movingUp = false;
    boolean movingLeft = false;
    boolean movingDown = false;
    boolean movingRight = false;
    boolean spaceBarDown = false;

    // FPS Counter
    final int fpsTextSize = 20;
    final int fpsColor = color(0, 255, 0);
    final int fpsMargin = 10;
    PFont fpsFont;

    // ------------------------------- END OF VARIABLES ------------------------------- //


    // ------------------------------------ SETUP ------------------------------------- //
    public void settings() {
        size(windowWidth, windowHeight, P2D);
    }

    public void setup() {
        surface.setTitle(gameTitle);
        frameRate(framesPerSecond);
        ellipseMode(CORNER);
        fpsFont = createFont("Arial Bold", fpsTextSize);

        loadVehicles();
        placeRoads();
        placePlayerAtStart();

        nextRoadOneSpawn = millis();
        nextRoadTwoSpawn = millis() + roadTwoSpawnOffset;
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
        movePlayer(playerSpeed);
        spawnVehicles();
        moveVehicles();
        checkCollisions();
    }


    // ---------------------------------- MAIN DRAW ----------------------------------- //
    public void drawGame() {
        background(backgroundColor);
        drawRoads();
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
    public void placeRoads() {
        int centerY = height / 2;

        roadOneY = centerY - roadGap / 2 - roadHeight;
        roadTwoY = centerY + roadGap / 2;
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


    // -------------------------------- VEHICLE SYSTEM -------------------------------- //
    public void loadVehicles() {
        // Build every vehicle once so spawning later is instant
        for (int type = 0; type < vehicleNames.length; type++) {
            leftVehicles[type] = loadVehicle(vehicleNames[type] + "_left.png");
            rightVehicles[type] = loadVehicle(vehicleNames[type] + "_right.png");
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
            if (vehicle != null && isPlayerHitting(vehicle)) {
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
        return alpha(pixelColor) >= minVisibleAlpha;
    }


    // ---------------------------------- DEBUG MODE ---------------------------------- //
    public PImage makeHitboxImage(PImage vehicleImage) {
        PImage hitbox = createImage(vehicleImage.width, vehicleImage.height, ARGB);
        hitbox.loadPixels();

        // Paint every visible vehicle pixel in the hitbox color
        for (int index = 0; index < vehicleImage.pixels.length; index++) {
            if (alpha(vehicleImage.pixels[index]) >= minVisibleAlpha) {
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
        ellipse(playerX, playerY, playerSize, playerSize);
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

    public void boostPlayer() {
        boolean isMoving = movingUp || movingLeft || movingDown || movingRight;

        if (millis() - lastBoostTime < boostCooldown || !isMoving) {
            return;
        }

        movePlayer(boostSpeed);
        lastBoostTime = millis();
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

        // --- Boost (once per press) ---
        if (key == ' ' && !spaceBarDown) {
            spaceBarDown = true;
            boostPlayer();
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
        PApplet.main("frogger");
    }


    // -------------------------------- VEHICLE CLASS --------------------------------- //
    class Vehicle {
        PImage image;
        PImage hitbox;
        int x = 0;
        int y = 0;
        int speed = 0;

        Vehicle(PImage image, PImage hitbox) {
            this.image = image;
            this.hitbox = hitbox;
        }
    }
}

// --------------------------------- THE END OF CODE ---------------------------------- //
