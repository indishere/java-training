# Pongie native rewrite

This is a standalone Windows C++ rewrite of the Java Pong game. It uses the
Win32 window and GDI APIs directly, limits updates to 60 FPS, sleeps between
frames, and redraws only the moving objects.

The opponent predicts the ball's landing point locally; this version does not
use the Java game's optional Cloudflare API integration or require Java.

## Build and run

With LLVM-MinGW or MinGW-w64 (`clang++` or `g++`) on `PATH`, run:

```powershell
.\build.ps1
.\pong-cpp.exe
```

The build statically links its C++ runtime. It uses Windows' built-in GDI and
user-interface libraries.

## Controls

- **E** or **Up Arrow**: move your paddle up
- **D** or **Down Arrow**: move your paddle down
- **F11**: toggle fullscreen
- **Escape**: exit fullscreen
