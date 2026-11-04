# Java `pong.exe` vs Rust `pong-rs.exe`

Same game, same rules. Java ships a bundled JVM; Rust calls Windows directly.

| | Java | Rust |
|---|---|---|
| Exe size | 26 MB | 15 KB |
| Disk after first run | 253 MB | 15 KB |
| Startup | 1.0–1.9 s | 0.12 s |
| CPU | 29–42 % | 0.1–5 % |
| Memory (working set / private) | 145 MB / 160 MB | 12 MB / 1.6 MB |
| Threads | 44 | 4 |

i5-6500, 1080p, Windows 11, warm starts, 3 runs each.

## Why

- No runtime: `no_std`, no CRT, imports only `user32`, `gdi32`, `kernel32`, `winhttp`.
- Redraws only what moved, nothing on idle screens. Java repaints the full window 60×/s.
- High-res waitable timer instead of `Thread.sleep`, so it holds a steady 60 FPS.
- Plans the AI move on serve or paddle hit, so no 10 ms polling thread.
- Model call runs off-thread over one reused WinHTTP connection (70–90 ms vs 200+ ms).
- No allocations: fixed buffers, one `static` game state.

## Differences

- Rust reads `.env` at runtime. Java bakes the keys into the exe.
- Rust checks certificate revocation and retries without the check only if the list
  can't be reached.
- Needs Windows 10 1803+ for the timer and 8.1+ for aiming.
