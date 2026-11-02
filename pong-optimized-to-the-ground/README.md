# Pongie native rewrite (Rust)

A standalone Windows Rust rewrite of the Java Pong game in `../pong`, with the same
gameplay and the same Cloudflare clef-flash aiming. It calls Windows' window and
GDI APIs directly, with no crates, limits updates to 60 FPS, and redraws only the
moving objects. See [OPTIMIZATIONS.md](OPTIMIZATIONS.md) for how it compares to
the Java `pong.exe`.

## Build and run

Needs Rust (`cargo`) with a MinGW/LLVM-MinGW Windows target. Then:

```powershell
.\build.ps1
.\pong-rs.exe
```

The exe links its C runtime statically and needs only DLLs that ship with Windows 10+.

For the opponent to aim with clef-flash, put a `.env` next to the exe (or in the
folder you launch it from), the same file the Java game uses:

```
CLOUDFLARE_CLEF_API_KEY=...
CLOUDFLARE_ACCOUNT_ID=...
```

Without it the opponent still defends; it just doesn't aim.

## Tests

```powershell
cargo test --release                    # game logic, offline
cargo test --release -- --include-ignored  # also calls clef-flash using ../pong/.env
```

## Controls

- **E** or **Up Arrow**: move your paddle up
- **D** or **Down Arrow**: move your paddle down
- **F11**: toggle fullscreen
- **Escape**: exit fullscreen
