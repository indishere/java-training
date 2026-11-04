# Pongie native rewrite (Rust)

A standalone Windows Rust rewrite of the Java Pong game in `../pong`, with the same
gameplay and the same Cloudflare clef-flash aiming. It calls Windows' window, GDI
and WinHTTP APIs directly, with no crates, no Rust standard library and no C runtime
(the exe is 15 KB), limits updates to 60 FPS, and redraws only the moving objects. See [OPTIMIZATIONS.md](OPTIMIZATIONS.md) for how it compares to
the Java `pong.exe`.

## Build and run

Needs Rust (`cargo`) and `rustup target add x86_64-pc-windows-msvc`. Nothing else: no
Visual Studio, no MinGW (it links with Rust's bundled `rust-lld`). Then:

```powershell
.\build.ps1
.\pong-rs.exe
```

The exe imports only from `user32`, `gdi32`, `kernel32` and `winhttp`, which ship with Windows.

For the opponent to aim with clef-flash, put a `.env` next to the exe (or in the
folder you launch it from), the same file the Java game uses:

```
CLOUDFLARE_CLEF_API_KEY=...
CLOUDFLARE_ACCOUNT_ID=...
```

Without it the opponent still defends; it just doesn't aim.

## Tests

Tests build with `std`, so they need a toolchain that can link normal programs: the
`stable-x86_64-pc-windows-gnu` host toolchain works out of the box (or MSVC with Visual Studio).

```powershell
cargo test --release                    # game logic, offline
cargo test --release -- --include-ignored  # also calls clef-flash using ../pong/.env
```

## Controls

- **E** or **Up Arrow**: move your paddle up
- **D** or **Down Arrow**: move your paddle down
- **F11**: toggle fullscreen
- **Escape**: exit fullscreen
