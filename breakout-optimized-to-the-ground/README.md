# Breakout native rewrite (Rust)

A standalone Windows Rust rewrite of the Java Breakout game in `../breakout`, with the
same gameplay: 3 lives (now stamped from the Minecraft hardcore-heart sprite), 5 levels
from the `BrickGrid.java` layout tables, 60 FPS fixed cadence, 35° launch, position-based
60° paddle bounce, and the same brick resolution order. It calls Windows' window and GDI
APIs directly, with no crates, no Rust standard library and no C runtime (the exe is
~15 KB), and redraws only the dirty region. The brick wall shades as one continuous
light-blue gradient instead of Java's flat blue. See
[OPTIMIZATIONS.md](OPTIMIZATIONS.md) for how it compares to the Java `breakout.exe`.

## Build and run

Needs Rust (`cargo`). Nothing else: `build.ps1` probes for an MSVC toolchain's standard
library (that links with Rust's bundled `rust-lld`) and falls back to a `gnullvm`
toolchain via `llvm-mingw` if that's what's installed. Then:

```powershell
.\build.ps1
.\breakout-rs.exe
```

The exe imports only from `user32`, `gdi32`, `kernel32` (and, transitively, `ntdll`),
which ship with Windows.

## Tests

`cargo test` builds with `std`, so it needs a toolchain that can link ordinary test
programs (the `gnullvm` host toolchain works out of the box, or MSVC with Visual
Studio):

```powershell
cargo test --release        # 7 tests: sin/cos series, launch, paddle bounce,
                            # brick collide + scoring, overlay texts, level
                            # layouts, paddle movement/clamps
```

## Controls

- **Mouse**, **←** / **→** or **A** / **D**: move the paddle
- **Space** or **Enter**: launch the ball / start a new game after GAME OVER or YOU WIN
- **P**: pause and continue
- **R**: restart from any state
- No fullscreen toggle: the Java game has none
