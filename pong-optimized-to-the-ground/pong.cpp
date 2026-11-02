#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <string>

namespace {

constexpr int kFps = 60;
constexpr int kBallSize = 30;
constexpr int kPaddleWidth = 15;
constexpr int kPaddleHeight = 100;
constexpr double kStartSpeed = 7.0;
constexpr double kSpeedUp = 1.05;
constexpr wchar_t kWindowClass[] = L"PongCppWindow";
constexpr wchar_t kWindowTitle[] = L"Pongie (C++)";

enum class GameState { Countdown, Playing, GameOver };

struct Paddle {
    double x = 20;
    double y = 0;
};

struct Ball {
    double x = 0;
    double y = 0;
    double speedX = kStartSpeed;
    double speedY = kStartSpeed;
};

struct Game {
    HWND window = nullptr;
    int width = 800;
    int height = 600;
    Paddle player;
    Paddle opponent;
    Ball ball;
    double opponentTargetY = 0;
    GameState state = GameState::Countdown;
    int timer = 3 * kFps;
    int messageTimer = 0;
    std::wstring message = L"3";
    bool movingUp = false;
    bool movingDown = false;
    bool fullscreen = false;
    LONG_PTR windowedStyle = 0;
    RECT windowedRect{};
    HFONT font = nullptr;
    HDC backBufferDC = nullptr;
    HBITMAP backBufferBitmap = nullptr;
    HGDIOBJ originalBitmap = nullptr;

    void reset() {
        player = {20, (height - kPaddleHeight) / 2.0};
        opponent = {static_cast<double>(width - 35), (height - kPaddleHeight) / 2.0};
        ball = {(width - kBallSize) / 2.0, (height - kBallSize) / 2.0,
                kStartSpeed, kStartSpeed};
        if (GetTickCount64() & 1) {
            ball.speedX = -ball.speedX;
        }
        aim();
    }

    // Plan once per approach, like the Java engine thread, not every frame.
    void aim() {
        opponentTargetY = ball.speedX > 0 ? predictLandingY() : height / 2.0;
    }

    void invalidateSprite(double x, double y, int spriteWidth, int spriteHeight) const {
        RECT area{
            static_cast<LONG>(std::floor(x)) - 1,
            static_cast<LONG>(std::floor(y)) - 1,
            static_cast<LONG>(std::ceil(x + spriteWidth)) + 1,
            static_cast<LONG>(std::ceil(y + spriteHeight)) + 1
        };
        InvalidateRect(window, &area, FALSE);
    }

    double predictLandingY() const {
        double x = ball.x;
        double y = ball.y;
        double speedY = ball.speedY;
        while (x + kBallSize < opponent.x) {
            x += ball.speedX;
            y += speedY;
            if (y <= 0 && speedY < 0) speedY = -speedY;
            if (y + kBallSize >= height && speedY > 0) speedY = -speedY;
        }
        return y + kBallSize / 2.0;
    }

    static bool overlaps(const Ball& b, const Paddle& p) {
        return b.x < p.x + kPaddleWidth && b.x + kBallSize > p.x
            && b.y < p.y + kPaddleHeight && b.y + kBallSize > p.y;
    }

    void hitPaddle(const Paddle& paddle) {
        const double ballMiddle = ball.y + kBallSize / 2.0;
        const double paddleMiddle = paddle.y + kPaddleHeight / 2.0;
        const double hitSpot = std::clamp(
            (ballMiddle - paddleMiddle) / (kPaddleHeight / 2.0), -1.0, 1.0);
        ball.speedX = -ball.speedX * kSpeedUp;
        ball.speedY = hitSpot * std::abs(ball.speedX);
        aim();
    }

    bool update() {
        if (state == GameState::Playing) {
            invalidateSprite(player.x, player.y, kPaddleWidth, kPaddleHeight);
            invalidateSprite(opponent.x, opponent.y, kPaddleWidth, kPaddleHeight);
            invalidateSprite(ball.x, ball.y, kBallSize, kBallSize);

            const double speed = std::abs(ball.speedX);
            const double paddleSpeed = std::round(speed); // Java paddles move whole pixels
            if (movingUp) player.y -= paddleSpeed;
            if (movingDown) player.y += paddleSpeed;
            player.y = std::clamp(player.y, 0.0,
                                  std::max(0.0, static_cast<double>(height - kPaddleHeight)));

            const double opponentMiddle = opponent.y + kPaddleHeight / 2.0;
            if (opponentMiddle > opponentTargetY + speed) opponent.y -= paddleSpeed;
            if (opponentMiddle < opponentTargetY - speed) opponent.y += paddleSpeed;
            opponent.y = std::clamp(opponent.y, 0.0,
                                    std::max(0.0, static_cast<double>(height - kPaddleHeight)));

            ball.x += ball.speedX;
            ball.y += ball.speedY;
            if (ball.y <= 0 && ball.speedY < 0) ball.speedY = -ball.speedY;
            if (ball.y + kBallSize >= height && ball.speedY > 0) ball.speedY = -ball.speedY;

            if (overlaps(ball, player) && ball.speedX < 0 && ball.x > player.x) {
                hitPaddle(player);
            }
            if (overlaps(ball, opponent) && ball.speedX > 0 && ball.x < opponent.x) {
                hitPaddle(opponent);
            }

            if (ball.x <= 0 || ball.x + kBallSize >= width) {
                reset();
                state = GameState::GameOver;
                message = L"GAME OVER";
                timer = 3 * kFps;
                return true;
            }
            invalidateSprite(player.x, player.y, kPaddleWidth, kPaddleHeight);
            invalidateSprite(opponent.x, opponent.y, kPaddleWidth, kPaddleHeight);
            invalidateSprite(ball.x, ball.y, kBallSize, kBallSize);
            if (messageTimer > 0 && --messageTimer == 0) {
                message.clear();
                return true;
            }
            return false;
        }

        --timer;
        if (state == GameState::GameOver) {
            if (timer == 0) {
                state = GameState::Countdown;
                message = L"3";
                timer = 3 * kFps;
                return true;
            }
            return false;
        }

        if (timer == 2 * kFps) {
            message = L"2";
            return true;
        }
        if (timer == kFps) {
            message = L"1";
            return true;
        }
        if (timer == 0) {
            state = GameState::Playing;
            message = L"GO!";
            messageTimer = kFps;
            return true;
        }
        if (messageTimer > 0 && --messageTimer == 0) {
            message.clear();
            return true;
        }
        return false;
    }

    void paint(HDC target) {
        RECT area{0, 0, width, height};
        FillRect(target, &area, static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH)));

        // ponytail: GAME OVER overlay (black at alpha 180) pre-multiplied into the
        // sprite colours; the background is already black so no AlphaBlend needed.
        const int keep = state == GameState::GameOver ? 255 - 180 : 255;
        auto shade = [keep](int r, int g, int b) {
            return RGB(r * keep / 255, g * keep / 255, b * keep / 255);
        };

        HBRUSH dcBrush = static_cast<HBRUSH>(GetStockObject(DC_BRUSH));
        SetDCBrushColor(target, shade(0, 80, 255));
        RECT left{static_cast<LONG>(player.x), static_cast<LONG>(player.y),
                  static_cast<LONG>(player.x) + kPaddleWidth,
                  static_cast<LONG>(player.y) + kPaddleHeight};
        RECT right{static_cast<LONG>(opponent.x), static_cast<LONG>(opponent.y),
                   static_cast<LONG>(opponent.x) + kPaddleWidth,
                   static_cast<LONG>(opponent.y) + kPaddleHeight};
        FillRect(target, &left, dcBrush);
        FillRect(target, &right, dcBrush);

        SetDCBrushColor(target, shade(255, 0, 0));
        SetDCPenColor(target, shade(255, 0, 0));
        HGDIOBJ oldBrush = SelectObject(target, dcBrush);
        HGDIOBJ oldPen = SelectObject(target, GetStockObject(DC_PEN));
        Ellipse(target, static_cast<int>(ball.x), static_cast<int>(ball.y),
                static_cast<int>(ball.x) + kBallSize,
                static_cast<int>(ball.y) + kBallSize);
        SelectObject(target, oldPen);
        SelectObject(target, oldBrush);

        if (!message.empty()) {
            HGDIOBJ oldFont = SelectObject(target, font);
            SetTextColor(target, RGB(255, 255, 255));
            SetBkMode(target, TRANSPARENT);
            DrawTextW(target, message.c_str(), -1, &area,
                      DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            SelectObject(target, oldFont);
        }
    }
};

Game game;

void releaseBackBuffer() {
    if (game.backBufferDC) {
        if (game.originalBitmap) SelectObject(game.backBufferDC, game.originalBitmap);
        if (game.backBufferBitmap) DeleteObject(game.backBufferBitmap);
        DeleteDC(game.backBufferDC);
        game.backBufferDC = nullptr;
        game.backBufferBitmap = nullptr;
        game.originalBitmap = nullptr;
    }
}

void createBackBuffer(HWND window) {
    releaseBackBuffer();
    HDC screen = GetDC(window);
    game.backBufferDC = CreateCompatibleDC(screen);
    game.backBufferBitmap = CreateCompatibleBitmap(screen, game.width, game.height);
    ReleaseDC(window, screen);
    if (!game.backBufferDC || !game.backBufferBitmap) {
        releaseBackBuffer();
        return;
    }
    game.originalBitmap = SelectObject(game.backBufferDC, game.backBufferBitmap);
}

void toggleFullscreen() {
    if (!game.fullscreen) {
        game.windowedStyle = GetWindowLongPtrW(game.window, GWL_STYLE);
        GetWindowRect(game.window, &game.windowedRect);
        MONITORINFO monitor{sizeof(monitor)};
        GetMonitorInfoW(MonitorFromWindow(game.window, MONITOR_DEFAULTTONEAREST), &monitor);
        SetWindowLongPtrW(game.window, GWL_STYLE,
                          game.windowedStyle & ~static_cast<LONG_PTR>(WS_OVERLAPPEDWINDOW));
        SetWindowPos(game.window, HWND_TOP, monitor.rcMonitor.left, monitor.rcMonitor.top,
                     monitor.rcMonitor.right - monitor.rcMonitor.left,
                     monitor.rcMonitor.bottom - monitor.rcMonitor.top,
                     SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        game.fullscreen = true;
    } else {
        SetWindowLongPtrW(game.window, GWL_STYLE, game.windowedStyle);
        SetWindowPos(game.window, nullptr, game.windowedRect.left, game.windowedRect.top,
                     game.windowedRect.right - game.windowedRect.left,
                     game.windowedRect.bottom - game.windowedRect.top,
                     SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        game.fullscreen = false;
    }
}

LRESULT CALLBACK windowProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_SIZE: {
        game.width = std::max(1, static_cast<int>(LOWORD(lParam)));
        game.height = std::max(1, static_cast<int>(HIWORD(lParam)));
        createBackBuffer(window);
        game.opponent.x = game.width - 35; // keep it on the right edge
        if (game.state != GameState::Playing) game.reset();
        InvalidateRect(window, nullptr, FALSE);
        return 0;
    }
    case WM_ERASEBKGND:
        return 1;
    case WM_PAINT: {
        HRGN dirtyRegion = CreateRectRgn(0, 0, 0, 0);
        const int regionType = GetUpdateRgn(window, dirtyRegion, FALSE);
        PAINTSTRUCT paint{};
        HDC screen = BeginPaint(window, &paint);
        if (game.backBufferDC) {
            if (regionType > NULLREGION) {
                SelectClipRgn(game.backBufferDC, dirtyRegion);
            }
            game.paint(game.backBufferDC);
            const RECT& dirty = paint.rcPaint;
            BitBlt(screen, dirty.left, dirty.top, dirty.right - dirty.left,
                   dirty.bottom - dirty.top, game.backBufferDC, dirty.left, dirty.top, SRCCOPY);
            SelectClipRgn(game.backBufferDC, nullptr);
        }
        EndPaint(window, &paint);
        DeleteObject(dirtyRegion);
        return 0;
    }
    case WM_KEYDOWN:
        if ((lParam & (1LL << 30)) == 0) {
            if (wParam == VK_F11) toggleFullscreen();
            if (wParam == VK_ESCAPE && game.fullscreen) toggleFullscreen();
        }
        if (wParam == 'E' || wParam == VK_UP) game.movingUp = true;
        if (wParam == 'D' || wParam == VK_DOWN) game.movingDown = true;
        return 0;
    case WM_KEYUP:
        if (wParam == 'E' || wParam == VK_UP) game.movingUp = false;
        if (wParam == 'D' || wParam == VK_DOWN) game.movingDown = false;
        return 0;
    case WM_KILLFOCUS:
        game.movingUp = false;
        game.movingDown = false;
        return 0;
    case WM_DESTROY:
        releaseBackBuffer();
        PostQuitMessage(0);
        return 0;
    default:
        return DefWindowProcW(window, message, wParam, lParam);
    }
}

} // namespace

#ifndef CREATE_WAITABLE_TIMER_HIGH_RESOLUTION
#define CREATE_WAITABLE_TIMER_HIGH_RESOLUTION 0x00000002
#endif

int WINAPI WinMain(HINSTANCE instance, HINSTANCE, LPSTR, int) {
    WNDCLASSW windowClass{};
    windowClass.lpfnWndProc = windowProc;
    windowClass.hInstance = instance;
    windowClass.hCursor = LoadCursorW(nullptr, MAKEINTRESOURCEW(32512));
    windowClass.hbrBackground = static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH));
    windowClass.lpszClassName = kWindowClass;
    if (!RegisterClassW(&windowClass)) return 1;

    game.font = CreateFontW(72, 0, 0, 0, FW_BOLD, FALSE, FALSE, FALSE,
                            DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                            CLEARTYPE_QUALITY, DEFAULT_PITCH, L"Arial");
    game.window = CreateWindowExW(
        0, kWindowClass, kWindowTitle, WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT, 800, 600, nullptr, nullptr, instance, nullptr);
    if (!game.window) {
        DeleteObject(game.font);
        return 1;
    }

    game.reset();
    ShowWindow(game.window, SW_MAXIMIZE);
    UpdateWindow(game.window);

    using Clock = std::chrono::steady_clock;
    constexpr auto frameDuration = std::chrono::microseconds(1000000 / kFps);
    auto nextFrame = Clock::now();
    // Sub-millisecond waits on Windows 10 1803+; plain ~15ms sleeps otherwise.
    HANDLE frameTimer = CreateWaitableTimerExW(
        nullptr, nullptr, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS);
    MSG message{};
    bool running = true;
    while (running) {
        auto now = Clock::now();
        if (now < nextFrame && frameTimer) {
            LARGE_INTEGER due{};
            due.QuadPart = -std::chrono::duration_cast<std::chrono::nanoseconds>(
                nextFrame - now).count() / 100;
            SetWaitableTimer(frameTimer, &due, 0, nullptr, nullptr, FALSE);
            MsgWaitForMultipleObjects(1, &frameTimer, FALSE, INFINITE, QS_ALLINPUT);
        } else if (now < nextFrame) {
            const auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(
                nextFrame - now);
            const DWORD wait = static_cast<DWORD>(
                std::max<int64_t>(1, remaining.count()));
            MsgWaitForMultipleObjects(0, nullptr, FALSE, wait, QS_ALLINPUT);
        }

        while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
            if (message.message == WM_QUIT) {
                running = false;
                break;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if (!running) break;

        now = Clock::now();
        if (now >= nextFrame) {
            if (game.update()) InvalidateRect(game.window, nullptr, FALSE);
            nextFrame += frameDuration;
            if (nextFrame <= now) nextFrame = now + frameDuration; // fell behind: don't burst
        }
    }

    if (frameTimer) CloseHandle(frameTimer);
    DeleteObject(game.font);
    return static_cast<int>(message.wParam);
}
