# Java `breakout.exe` vs Rust `breakout-rs.exe`

Same game, same rules. Java ships a bundled JVM and repainted Swing; Rust calls Windows
directly.

## Benchmark

i5-6500, HD 530, 1920×1080, Windows 11. 3 runs each: 8 s on READY, then 12 s of play
(Space pressed by SendKeys). Java counts both processes (launcher + `javaw`); its runtime
was already extracted to `%LOCALAPPDATA%\BreakOut`, so no cold unpack is included.

| | Java | Rust | |
|---|---|---|---|
| Exe size | 25.5 MB (25,474,048 B) | 15,872 B | ~1,600× |
| Disk after first run | 25.5 MB + 232 MB runtime (417 files) | 15.5 KB | ~16,000× |
| Startup | 0.8–6.2 s | 0.11–0.12 s | ~7–54× |
| CPU, READY (% of one core) | 1.8–8.2 % | 0.4–1.0 % | ~5× |
| CPU, playing | ~5.9 % | 1.0–2.7 % | ~2–6× |
| Working set | ~62 MB | ~11.8 MB | ~5× |
| Private memory | ~206 MB | ~1.6 MB | ~100× |
| Processes / threads | 2 / 36 | 1 / 4 | |
| Handles | ~624 | 144 | |

All Rust runs here are the final `no_std` build. One cold-start outlier at 1,035 ms
(disk cache miss) was excluded from the Rust startup range; warm starts were 109–124 ms.

## Why

- No runtime: `no_std`, no C runtime, imports only `user32`, `gdi32`, `kernel32`
  (`ntdll` is mapped into every process anyway, and only what the optimizer emits gets imported).
- Redraws only what moved: the dirty region (`GetUpdateRgn` + `SelectClipRgn`) bounds every
  paint call, and `InvalidateRect` is called only on the ball/paddle boxes that changed.
  Java's Swing `repaint()` hits the full window 60×/s no matter what moved.
- Overlay states (READY/PAUSED/LEVEL CLEARED/GAME OVER/YOU WIN) paint once per state
  change; Java repainted them 60×/s like any other frame.
- Brick bounds derive from one division per column (`reflow`), so collision walks
  arrays instead of Java's list of 50 `Rectangle`s.
- The wall's "is cleared" answer is one counter (`bricks_left`) instead of Java's
  list rescan on every hit.
- Zero allocations at runtime: one `static` game state, fixed buffers for overlay text,
  fonts and regions created once.
- `Math.sin`/`cos` are 8-term power series: the ball only ever needs |angle| ≤ 60°, so
  no libm and no range reduction (~1e-13 error over that interval).
- A high-resolution waitable timer plus `QueryPerformanceCounter` holds the 60 FPS
  cadence; `MsgWaitForMultipleObjects` gates every frame.

## Differences

- The lives row stamps the Minecraft hardcore-heart sprite supplied by the player
  (13×12 cells: red body, dark-maroon shading, black outline, 2-cell tip). Java drew
  its own segmented hearts. Lost lives keep the sprite's shape, greyed out.
- The brick wall shades as one continuous light-blue gradient (lightest at the top row,
  deepest at the bottom) instead of Java's flat `Color.BLUE` per brick.
- No fullscreen toggle: the Java game has no F11 mode, so none was added (unlike the
  Pong rewrite).
- `WM_KILLFOCUS` drops held keys so the paddle can't glide on by itself when the
  window loses focus mid-press.
- The window's minimum size is 640×480 (Java: `frame.setMinimumSize`), enforced by
  `WM_GETMINMAXINFO`.
- The title is `Breakout (Rust)` so the two windows are told apart on screen.
