# Java `pong.exe` vs Rust `pong-rs.exe`

Same game, same rules. Java ships a bundled JVM; Rust calls Windows directly.

## Benchmark

i5-6500, HD 530, 1920×1080, Windows 11. 3 warm runs each, keys loaded, 15 s of play
after the countdown. Java counts both processes (launcher + `javaw`).

| | Java | Rust | |
|---|---|---|---|
| Exe size | 26.0 MB | 15 KB | ~1,700× |
| Disk after first run | 26 MB + 227 MB runtime (448 files) | 15 KB | ~17,000× |
| Startup | 1.0–1.9 s | 0.12 s | ~10× |
| CPU (% of one core) | 29–42 % | 0.1–5.3 % | ~8–300× |
| Working set | 143–148 MB | 12 MB | ~12× |
| Private memory | 159–161 MB | 1.6 MB | ~100× |
| Processes / threads | 2 / 43–44 | 1 / 4 | |
| Handles | 800–824 | 143 | |

Runtime rows were measured on Rust v0.1 (0.4 MB); v0.2 measures the same (below).

## Why

- No runtime: `no_std`, no CRT, imports only `user32`, `gdi32`, `kernel32`, `winhttp`.
- Redraws only what moved, nothing on idle screens. Java repaints the full window 60×/s.
- High-res waitable timer instead of `Thread.sleep`, so it holds a steady 60 FPS.
- Plans the AI move on serve or paddle hit, so no 10 ms polling thread.
- Model call runs off-thread over one reused WinHTTP connection.
- No allocations: fixed buffers, one `static` game state.

## v0.1 (`std`, MinGW, `curl.exe`) → v0.2 (`no_std`, WinHTTP)

| | v0.1 | v0.2 |
|---|---|---|
| Exe size | 384,000 B | 15,360 B |
| Model request (first) | 183 ms | 191 ms |
| Model request (after) | 202–241 ms | 69–89 ms |
| TLS connections / 90 s | 4 | 1 |

Win 11 LTSC VM (6 vCPU, 8 GB, i7-14650HX host), 3 runs, no keys:

| | v0.1 | v0.2 |
|---|---|---|
| Startup | 24–31 ms | 15–36 ms |
| CPU | 0.2–1.5 % | 0.6–1.7 % |
| Working set / private | 15.1 / 1.6 MB | 15.2 / 1.6 MB |
| Threads / handles | 4 / 138–144 | 4 / 139 |

With keys, v0.2 shows 11–12 threads and ~330 handles because WinHTTP's pool runs in-process.
v0.1 ran a separate `curl.exe` per request instead, which isn't counted here.

## Differences

- Rust reads `.env` at runtime. Java bakes the keys into the exe.
- Rust checks certificate revocation and retries without the check only if the list
  can't be reached.
- Needs Windows 10 1803+ for the timer and 8.1+ for aiming.
