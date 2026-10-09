// IMPORTS
import processing.core.*;


// ====================================== FROGGER GAME ======================================= //
public class frogger extends PApplet {


    // ------------------------------ START OF VARIABLES ------------------------------ //
    // Colors are hex codes: 0xAARRGGBB (AA is how solid it is, FF = fully solid)

    // Window
    final String gameTitle = "Frogger DA Mogger";
    final int windowWidth = 1900;
    final int windowHeight = 1000;
    final int framesPerSecond = 60;
    final int backgroundColor = 0xFF000000; // black

    // Screens
    boolean loadingScreen = true;
    boolean levelScreen = false;
    boolean gameScreen = false;
    boolean gameOverScreen = false;

    // Player (Steve)
    final int playerSize = 75; // the size unit the roads and gaps are built from
    final int playerHeight = 135; // how tall Steve is (his width comes from the picture)
    final int playerSpeed = 3; // pixels Steve moves with each step
    final int stepDelay = 0; // milliseconds between steps while a key is held down
    final int playerStartGap = playerSize / 2;
    int lastStepTime = 0;
    PImage playerImage;
    PImage playerHitbox;
    int playerX = 0;
    int playerY = 0;

    // Jump (water and anything lower than the current jump height can't get the player)
    final int jumpLength = 240; // pixels flown if you hold a direction the whole jump
    final int jumpHeight = 60; // how high the player gets at the top of the jump
    final int jumpDuration = 250; // milliseconds in the air
    final int jumpFrames = jumpDuration * framesPerSecond / 1000; // frames in the air
    final int jumpCooldown = 1000;
    final int jumpGrowth = playerHeight / 5; // how much bigger the player looks at the top
    boolean isJumping = false;
    int jumpFrame = 0; // how many frames into the jump
    int jumpDistanceLeft = 0;
    int currentJumpHeight = 0; // 0 = on the ground
    int jumpStartTime = -jumpCooldown;

    // Roads
    final int roadHeight = 150;
    final int laneGap = playerHeight; // empty space between the roads and the river (fits Steve)
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
    final int[] minVehicleSpeeds = { 5, 3, 3 }; // pixels per frame
    final int[] maxVehicleSpeeds = { 10, 7, 6 }; // pixels per frame
    final int[] vehicleTallness = { 40, 100, 90 }; // how tall each vehicle is off the ground
    final int vehiclePadding = 5;
    final int vehicleHeight = roadHeight - vehiclePadding * 2;
    final int vehicleGap = playerSize * 2; // closest a vehicle can get to the one in front of it
    final int maxVehicles = 10;
    Vehicle[] vehicles = new Vehicle[maxVehicles];
    Vehicle[] leftVehicles = new Vehicle[vehicleNames.length];
    Vehicle[] rightVehicles = new Vehicle[vehicleNames.length];

    // Spawning
    final int minSpawnDelay = 2000;
    final int maxSpawnDelay = 4500;
    final int roadTwoSpawnOffset = 1000;
    Vehicle lastRoadOneVehicle = null;
    Vehicle lastRoadTwoVehicle = null;
    Vehicle nextRoadOneVehicle = null; // picked ahead of time so its speed is known before it spawns
    Vehicle nextRoadTwoVehicle = null;
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
    PFont fpsFont = null;

    // Loading
    boolean imagesLoaded = false;

    // ------------------------------- END OF VARIABLES ------------------------------- //


    // ------------------------------------ SETUP ------------------------------------- //
    public void settings() {
        size(windowWidth, windowHeight, P2D);
        noSmooth(); // keeps the pixel art sharp when Steve is drawn bigger mid-jump

        // Load the images while the window is still opening
        thread("loadImages");
    }

    public void setup() {
        surface.setTitle(gameTitle);
        frameRate(framesPerSecond);

        // Make the font in the background so the game can start without waiting for it
        thread("loadFont");

        placeLanes();

        // Wait in case the images are still loading
        while (!imagesLoaded) {
            delay(10);
        }

        placePlayerAtStart();

        nextRoadOneVehicle = makeVehicle(roadOneY, true);
        nextRoadTwoVehicle = makeVehicle(roadTwoY, false);
        nextRoadOneSpawn = millis();
        nextRoadTwoSpawn = millis() + roadTwoSpawnOffset;
        nextLogSpawn = millis();
        nextLogSpeed = (int) random(minLogSpeed, maxLogSpeed + 1);
    }

    public void loadImages() {
        loadPlayer();
        loadVehicles();
        loadLogs();
        imagesLoaded = true;
    }

    public void placePlayerAtStart() {
        playerX = (width - playerImage.width) / 2;
        playerY = height - playerImage.height - playerStartGap;
    }


    // ---------------------------------- GAME LOOP ----------------------------------- //
    public void draw() {
        updateGame();
        drawGame();
    }

    public void updateGame() {
        if (isJumping) {
            updateJump();
        } else {
            keepWalking();
        }

        spawnVehicles();
        moveVehicles();
        spawnLogs();
        moveLogs();

        checkCollisions();

        // Water can't get the player while they're in the air
        if (currentJumpHeight == 0) {
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
    public void loadFont() {
        fpsFont = createFont("Arial Bold", fpsTextSize);
    }

    public void drawFps() {
        // The font is still loading for the first few frames
        if (fpsFont == null) {
            return;
        }

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

        // The last log is still floating in, so there's no room yet
        if (lastLog.x < logGap) {
            return false;
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
        int playerCenterX = playerX + playerImage.width / 2;
        int playerCenterY = playerY + playerImage.height / 2;

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

        // No log under the player: they fell in the water
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
        if (millis() >= nextRoadOneSpawn && hasRoomFor(nextRoadOneVehicle, lastRoadOneVehicle)) {
            addVehicle(nextRoadOneVehicle);
            lastRoadOneVehicle = nextRoadOneVehicle;
            nextRoadOneVehicle = makeVehicle(roadOneY, true);
            nextRoadOneSpawn = millis() + (int) random(minSpawnDelay, maxSpawnDelay);
        }

        // Road Two (drives right to left)
        if (millis() >= nextRoadTwoSpawn && hasRoomFor(nextRoadTwoVehicle, lastRoadTwoVehicle)) {
            addVehicle(nextRoadTwoVehicle);
            lastRoadTwoVehicle = nextRoadTwoVehicle;
            nextRoadTwoVehicle = makeVehicle(roadTwoY, false);
            nextRoadTwoSpawn = millis() + (int) random(minSpawnDelay, maxSpawnDelay);
        }
    }

    public Vehicle makeVehicle(int roadY, boolean isMovingRight) {
        int type = (int) random(vehicleNames.length);
        int speed = (int) random(minVehicleSpeeds[type], maxVehicleSpeeds[type] + 1);

        Vehicle vehicle;

        if (isMovingRight) {
            vehicle = new Vehicle(rightVehicles[type].image, rightVehicles[type].hitbox);
            vehicle.x = -vehicle.image.width;
            vehicle.speed = speed;
        } else {
            vehicle = new Vehicle(leftVehicles[type].image, leftVehicles[type].hitbox);
            vehicle.x = width;
            vehicle.speed = -speed;
        }

        vehicle.y = roadY + vehiclePadding;
        vehicle.tallness = vehicleTallness[type];
        return vehicle;
    }

    public int getDistanceFromEdge(Vehicle vehicle) {
        // How far the back of the vehicle is from the edge it drove in from
        if (vehicle.speed > 0) {
            return vehicle.x;
        } else {
            return width - (vehicle.x + vehicle.image.width);
        }
    }

    public boolean hasRoomFor(Vehicle newVehicle, Vehicle vehicleAhead) {
        if (vehicleAhead == null || getDistanceFromEdge(vehicleAhead) > width) {
            return true;
        }

        int distanceAhead = getDistanceFromEdge(vehicleAhead);

        // The vehicle ahead is still driving in, so there's no room yet
        if (distanceAhead < vehicleGap) {
            return false;
        }

        // How much closer the new vehicle gets to the one ahead before that one drives off the screen
        int speedAhead = abs(vehicleAhead.speed);
        int framesLeft = (width - distanceAhead) / speedAhead;
        int distanceGained = (abs(newVehicle.speed) - speedAhead) * framesLeft;

        return distanceAhead - distanceGained >= vehicleGap;
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

            // The player is only safe while they're higher than the vehicle
            boolean isJumpingOver = currentJumpHeight > vehicle.tallness;

            if (!isJumpingOver && isPlayerHitting(vehicle)) {
                placePlayerAtStart();
                return;
            }
        }
    }

    public boolean isPlayerHitting(Vehicle vehicle) {
        // Quick check: skip vehicles whose box doesn't touch the player's box
        boolean boxesTouch = playerX < vehicle.x + vehicle.image.width
                && playerX + playerImage.width > vehicle.x
                && playerY < vehicle.y + vehicle.image.height
                && playerY + playerImage.height > vehicle.y;

        if (!boxesTouch) {
            return false;
        }

        // Pixel check: look for a screen pixel where both Steve AND the vehicle are visible
        for (int pixelY = playerY; pixelY < playerY + playerImage.height; pixelY++) {
            for (int pixelX = playerX; pixelX < playerX + playerImage.width; pixelX++) {
                boolean isOnPlayer = isVisiblePixel(playerImage, playerX, playerY, pixelX, pixelY);
                boolean isOnVehicle = isVisiblePixel(vehicle.image, vehicle.x, vehicle.y, pixelX, pixelY);

                if (isOnPlayer && isOnVehicle) {
                    return true;
                }
            }
        }

        return false;
    }

    public boolean isVisiblePixel(PImage image, int imageLeft, int imageTop, int pixelX, int pixelY) {
        int imageX = pixelX - imageLeft;
        int imageY = pixelY - imageTop;

        boolean isOnImage = imageX >= 0 && imageX < image.width
                && imageY >= 0 && imageY < image.height;

        if (!isOnImage) {
            return false;
        }

        int pixelColor = image.pixels[imageY * image.width + imageX];
        return getAlpha(pixelColor) >= minVisibleAlpha;
    }

    public int getAlpha(int pixelColor) {
        // The alpha is the AA part of 0xAARRGGBB, so shift it down and keep only those two digits
        return (pixelColor >> 24) & 0xFF;
    }


    // ---------------------------------- DEBUG MODE ---------------------------------- //
    public PImage makeHitboxImage(PImage image) {
        PImage hitbox = createImage(image.width, image.height, ARGB);
        hitbox.loadPixels();

        // Paint every visible pixel in the hitbox color
        for (int index = 0; index < image.pixels.length; index++) {
            if (getAlpha(image.pixels[index]) >= minVisibleAlpha) {
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

        // Logs: the player is safe while their center is inside one of these boxes
        for (Log log : logs) {
            if (log != null) {
                noFill();
                stroke(boxOutlineColor);
                rect(log.x, log.y, log.image.width, log.image.height);
            }
        }

        // Player: box for the quick check, red pixels for the real hitbox
        noFill();
        stroke(boxOutlineColor);
        rect(playerX, playerY, playerImage.width, playerImage.height);
        image(playerHitbox, playerX, playerY);
    }


    // --------------------------------- PLAYER CODE ---------------------------------- //
    public void loadPlayer() {
        playerImage = resizeSharp(loadImage(imageFolder + "steve/steve.png"), playerHeight);
        playerHitbox = makeHitboxImage(playerImage);
    }

    public void drawPlayer() {
        // The higher the jump, the bigger Steve looks (he's closer to the camera)
        int growth = jumpGrowth * currentJumpHeight / jumpHeight;
        int drawWidth = playerImage.width + growth;
        int drawHeight = playerImage.height + growth * 2;

        // Grow from the center so Steve stays in the same spot
        int drawX = playerX - (drawWidth - playerImage.width) / 2;
        int drawY = playerY - (drawHeight - playerImage.height) / 2;

        image(playerImage, drawX, drawY, drawWidth, drawHeight);
    }

    public void stepPlayer(int stepX, int stepY) {
        // No stepping in mid-air, the jump already moves Steve
        if (isJumping) {
            return;
        }

        playerX += stepX;
        playerY += stepY;
        checkEdges();
        lastStepTime = millis();
    }

    public void keepWalking() {
        boolean isHoldingKey = movingUp || movingLeft || movingDown || movingRight;

        // Holding a key down takes another step every stepDelay
        if (isHoldingKey && millis() - lastStepTime >= stepDelay) {
            movePlayer(playerSpeed);
            lastStepTime = millis();
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
        jumpFrame = 0;
        jumpDistanceLeft = jumpLength;
    }

    public void updateJump() {
        jumpFrame++;

        // Split the distance that's left evenly over the frames that are left,
        // so every jump is exactly jumpLength long with no pixels lost to rounding
        int framesLeft = jumpFrames - jumpFrame + 1;
        int step = jumpDistanceLeft / framesLeft;
        jumpDistanceLeft -= step;

        // Going diagonally moves on both axes at once, so shrink the step (1 / square root of 2)
        boolean isDiagonal = (movingUp || movingDown) && (movingLeft || movingRight);

        if (isDiagonal) {
            movePlayer(round(step / sqrt(2)));
        } else {
            movePlayer(step);
        }

        // The height follows an arc: 0 at take-off, jumpHeight halfway, 0 at landing
        currentJumpHeight = 4 * jumpHeight * jumpFrame * (jumpFrames - jumpFrame) / (jumpFrames * jumpFrames);

        // Land once all the jump frames are used up
        if (jumpFrame >= jumpFrames) {
            isJumping = false;
            currentJumpHeight = 0;
        }
    }


    // ------------------------------------ WALLS ------------------------------------- //
    public void checkEdges() {
        int rightEdge = width - playerImage.width;
        int bottomEdge = height - playerImage.height;

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
        // Take a step right away when a key goes down (keepWalking handles holding it)

        // --- E / Up Arrow ---
        if (keyCode == 'E' || keyCode == UP) {
            if (isPressed && !movingUp) {
                stepPlayer(0, -playerSpeed);
            }
            movingUp = isPressed;
        }

        // --- S / Left Arrow ---
        if (keyCode == 'S' || keyCode == LEFT) {
            if (isPressed && !movingLeft) {
                stepPlayer(-playerSpeed, 0);
            }
            movingLeft = isPressed;
        }

        // --- D / Down Arrow ---
        if (keyCode == 'D' || keyCode == DOWN) {
            if (isPressed && !movingDown) {
                stepPlayer(0, playerSpeed);
            }
            movingDown = isPressed;
        }

        // --- F / Right Arrow ---
        if (keyCode == 'F' || keyCode == RIGHT) {
            if (isPressed && !movingRight) {
                stepPlayer(playerSpeed, 0);
            }
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
        // Skip some of the graphics driver checks so the window opens faster
        System.setProperty("jogl.disable.opengles", "true"); // phone-style OpenGL
        System.setProperty("jogl.disable.openglarbcontext", "true"); // ~1 second of extra driver probing
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
