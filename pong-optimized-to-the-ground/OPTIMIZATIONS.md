# Optimizations: Java `pong.exe` vs Rust `pong-rs.exe`

Both are the same game: same rules, same controls, same Cloudflare clef-flash
aiming. This page compares the two Windows executables, side by side.

- **Java:** `pong/pong.exe`, built by `pong/compiler.ps1`. It is a C# launcher that
  unpacks a bundled Java runtime and then starts `javaw.exe` with the game's jar.
- **Rust:** `pong-optimized-to-the-ground/pong-rs.exe`, built by `build.ps1`. It is
  one native program that calls Windows directly.

## Measured

Each exe was launched 3 times. Both had the API keys loaded. Each run waited 4 s
(past the countdown) and then measured 15 s of play. Java's numbers include both
of its processes (launcher + `javaw`).

Machine: Intel Core i5-6500, Intel HD Graphics 530, 1920×1080, Windows 11.

| | Java `pong.exe` | Rust `pong-rs.exe` | Difference |
|---|---|---|---|
| File you ship | 26.0 MB | 15 KB (was 0.4 MB) | ~1,700× smaller |
| Disk used after first run | 26 MB + 227 MB unpacked runtime (448 files in `%LOCALAPPDATA%\Pongie`) | 15 KB, nothing unpacked | ~17,000× less |
| Time until the window shows (warm) | 1.0–1.9 s | 0.12 s | ~10× faster |
| CPU while running (% of one core) | 29–42 % | 0.1–5.3 % | ~8–300× less |
| Memory (working set) | 143–148 MB | 12 MB | ~12× less |
| Private memory | 159–161 MB | 1.6 MB | ~100× less |
| Processes / threads | 2 / 43–44 | 1 / 4 | |
| Handles | 800–824 | 143 | |

The runtime rows are from the first Rust version (0.4 MB, with `std`). The current
15 KB build measures the same at runtime; see [Round 2](#round-2-no-standard-library-15-kb).

CPU varies with how much of the measuring window was active play and how much was
the countdown or GAME OVER screen. Java redraws the whole screen 60 times a second
either way. Rust redraws only what moved, and nothing at all while the countdown
text is just sitting there.

The first launch of each fresh build is slower for both programs (Windows Defender
scans new files, and Java also unpacks its runtime), so only warm starts are compared.

## Where the savings come from

| Java does | Rust does instead | Why it matters |
|---|---|---|
| Ships a whole Java runtime (jlink'd JDK modules) and unpacks it on first run | Calls Windows' own `user32`/`gdi32` directly, statically linked, `strip`ped, LTO | No runtime to download, unpack, load or JIT-compile |
| Swing repaints the **whole window** every frame | Invalidates only the old and new rectangles of the 2 paddles and the ball, and blits only that dirty area from a back buffer | A 1920×1080 frame is ~2 million pixels; the moving parts are ~10 thousand |
| Paints even when nothing moves (countdown, GAME OVER) | Repaints only when the text changes | Idle screens cost nothing |
| `new Font("Arial", BOLD, 72)` on every paint | Creates the font once | No per-frame allocation and font lookup |
| GAME OVER draws a translucent full-screen rectangle (alpha blending every pixel, every frame) | Draws the sprites pre-darkened; the background is already black, so it looks identical | No blending at all |
| `Thread.sleep(1000 / 60)`, which is 16 ms plus the frame's own work, so the game drifts slower than 60 FPS, and Windows' default ~15.6 ms sleep resolution makes it jittery | High-resolution waitable timer aimed at a fixed 16.67 ms schedule; if it falls behind, it resyncs instead of bursting | Steady 60 FPS with ~0 CPU while waiting |
| Engine thread wakes every 10 ms forever to check whether the ball is coming | Plans once, on the exact event (serve or paddle hit) | No polling thread |
| Engine thread **blocks** on the HTTP call, so a slow reply can delay the next approach's defence | Defence is computed instantly; the model call runs on a short-lived thread and is ignored if it comes back for an old approach | Opponent never freezes waiting on the network |
| Java `HttpClient` + regex, inside the JVM | Windows' built-in WinHTTP (Schannel, the Windows certificate store, the system proxy), one connection kept open for the whole game | No HTTP library in the exe, no handshake after the first request |
| Game state is a `String` (`"playing"`) compared with `.equals` every frame | `enum` | Trivial, but free |
| Allocates a new `Ball` and two `Paddle`s on every reset | Overwrites fields in place | No garbage collector involved |

## What is identical (1:1)

- 60 updates per second, ball 30 px, paddles 15×100, start speed 7, +5 % per hit,
  45° max angle from where the ball hits the paddle.
- Paddles move by the ball's speed rounded to whole pixels; positions use Java's
  integer maths (`height / 2 - 50`, landing spot truncated to `int`).
- The opponent heads to the predicted landing spot as soon as the ball comes its
  way, then asks clef-flash (same prompt) whether to shift ±35 px to aim up or down.
  The shift is applied only if the reply arrives more than 15 frames before the
  ball does, and only one request is in flight at a time.
- Keys come from `.env` (`CLOUDFLARE_CLEF_API_KEY`, `CLOUDFLARE_ACCOUNT_ID`).
  Without keys the opponent just defends.
- Countdown 3-2-1-GO!, GAME OVER with a darkened screen for 3 s, 72 pt bold Arial
  centred with its baseline at mid-height, blue paddles, red ball.
- E/↑ and D/↓ to move, F11 toggles fullscreen, Esc leaves it and returns to an
  800×600 window centred on screen. Starts maximized; resizing re-centres everything
  unless a rally is in progress.

### Small, deliberate differences

- **Where `.env` is read from.** The Java build bakes `.env` *into* `pong.exe`, so the
  keys ship with the file. The Rust exe reads `.env` from the working folder, or the
  folder next to the exe, so the exe is safe to share.
- **Certificate revocation.** On networks that re-sign HTTPS (like school),
  Windows can't reach the proxy's revocation list. Java doesn't check revocation;
  Rust checks revocation and, only when the list can't be reached, retries once
  without the check (like curl's `--ssl-revoke-best-effort`). Revoked certificates
  are still rejected.
- **Serve direction.** Java uses `Math.random()`; Rust uses the parity of
  `GetTickCount64()`. Both are 50/50.
- **Requirements.** Windows 10 1803 or later for the high-resolution timer (older
  Windows falls back to a plain ~15 ms timer). Aiming uses WinHTTP's system-proxy
  mode, Windows 8.1 or later.

## Round 2: no standard library (15 KB)

The Rust exe was 0.4 MB, almost all of it Rust's standard library, the MinGW C runtime
and panic/formatting machinery the game never needs. Version 0.2 removes all of it.

| | v0.1 (`std`, MinGW, `curl.exe`) | v0.2 (`no_std`, no CRT, WinHTTP) |
|---|---|---|
| File you ship | 384,000 bytes | **15,360 bytes** (25× smaller) |
| What the exe links | Rust `std` + static MinGW CRT | nothing: imports from `user32`, `gdi32`, `kernel32`, `winhttp` only |
| Needed to build | Rust + a MinGW/LLVM-MinGW toolchain | Rust + `rustup target add x86_64-pc-windows-msvc` (no Visual Studio, no MinGW) |
| Model request (after the first) | 202–241 ms, new `curl.exe` process + TLS handshake every time | **69–89 ms**, the same TLS connection reused |
| Model request (first) | 183 ms | 191 ms (handshake) |
| TLS connections in 90 s of play | 4 (one per request) | 1 |

How:

- `#![no_std]` + `#![no_main]`: the entry point is `mainCRTStartup` itself, the panic
  handler is `ExitProcess(1)`, and `_fltused` is defined by hand. The release exe
  contains no panic messages or `core::fmt` code (checked with `grep` on the binary).
- Windows functions are declared with `#[link(kind = "raw-dylib")]`, so rustc writes the
  import table itself: no `.lib` import libraries, and Rust's bundled `rust-lld` links
  it (`.cargo/config.toml`). `build.rs` adds `/NODEFAULTLIB` for the exe only.
- `memcpy`/`memset`/`memmove`/`memcmp`, if the compiler emits them, are imported from
  `ntdll.dll` (mapped into every process anyway). At `opt-level = "z"` none are used,
  so there is no ntdll import at all.
- No allocator: strings are compile-time UTF-16 arrays (`w!`/`wz!` macros), the
  request body and headers go into fixed buffers, and game state is one `static`.
- The model call is WinHTTP on a short-lived thread that posts the answer back to the
  window as a `WM_APP` message (no channel, no per-frame polling).
- Smaller per-frame savings: a paddle that didn't move isn't redrawn, the dirty-region
  object is created once instead of every frame, and `TranslateMessage` is gone.
- `opt-level = "z"`: 15,360 bytes. `"s"` is 16,384 and `"3"` is 22,016. The frame loop
  does almost nothing, so speed is the same.

Measured in a Windows 11 LTSC 24H2 VM (KVM, 6 vCPU, 8 GB, i7-14650HX host), 3 runs
each, same method as above, no keys:

| | v0.1 | v0.2 |
|---|---|---|
| Time until the window shows (warm) | 24–31 ms | 15–36 ms |
| CPU while running | 0.2–1.5 % | 0.6–1.7 % |
| Working set / private memory | 15.1 MB / 1.6 MB | 15.2 MB / 1.6 MB |
| Threads / handles | 4 / 138–144 | 4 / 139 |

At runtime the two are the same: what's left is Windows' own DLLs, which neither
version can avoid. With keys, v0.2 shows 11–12 threads and ~330 handles because
WinHTTP's worker pool runs inside the game process; v0.1 did the same work in a
separate `curl.exe` process per request, which this measurement doesn't count.
