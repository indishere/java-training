# Frogger DA Mogger native rewrite (Rust)

A standalone Windows Rust rewrite of the Java Frogger game in `../frogger-game`,
with the same gameplay, controls and PNG assets. It calls Windows' window and
GDI APIs directly, with no crates, no Rust standard library and no C runtime
(the exe is 30 KB), decodes the PNGs itself, limits updates to 60 FPS through a
high-resolution waitable timer, and redraws only the areas that changed. See
[OPTIMIZATIONS.md](OPTIMIZATIONS.md) for how it compares to the Java game.

## Build and run

Needs Rust (`cargo`) and the toolchain already installed on this machine: a
standalone llvm-mingw-flavoured Rust (`x86_64-pc-windows-gnullvm`, no rustup).
Nothing else: no Visual Studio, no MinGW install beyond that toolchain (the
linker is its own lld, pointed at CRT-free link arguments by `build.rs`). Then:

```powershell
.\build.ps1
.\frogger-rs.exe
```

The exe imports only from `user32`, `gdi32` and `kernel32`, which ship with
Windows. The eleven PNGs load from `assets\images\` next to the exe at startup —
byte-for-byte the same files the Java sketch reads — so the folder carries the
assets like `../frogger-game` does.

Unit tests build with `std` from the same `main.rs` (`cargo test` compiles the
file as a lib through the normal import libraries), so they work with any host
toolchain that can link ordinary test executables.

```powershell
cargo test --release
```

## Controls

The game window is 1900×1000 (Java's `size()` 1900, 1000). Steve walks, jumps,
rides the logs, and dies the same ways as in the Java game.

- **E** or **Up Arrow**: move up
- **S** or **Left Arrow**: move left
- **F** or **Right Arrow**: move right
- **D** or **Down Arrow**: move down
- **Space**: jump (a 15-frame arc, Java's exact maths)
- **Shift+D**: toggle the debug overlay (hitboxes on vehicles, logs and Steve)
