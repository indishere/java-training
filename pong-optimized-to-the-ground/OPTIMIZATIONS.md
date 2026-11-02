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
| File you ship | 26.0 MB | 0.4 MB | 63× smaller |
| Disk used after first run | 26 MB + 227 MB unpacked runtime (448 files in `%LOCALAPPDATA%\Pongie`) | 0.4 MB, nothing unpacked | ~600× less |
| Time until the window shows (warm) | 1.0–1.9 s | 0.12 s | ~10× faster |
| CPU while running (% of one core) | 29–42 % | 0.1–5.3 % | ~8–300× less |
| Memory (working set) | 143–148 MB | 12 MB | ~12× less |
| Private memory | 159–161 MB | 1.6 MB | ~100× less |
| Processes / threads | 2 / 43–44 | 1 / 4 | |
| Handles | 800–824 | 143 | |

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
| Java `HttpClient` + regex, inside the JVM | Windows' built-in `curl.exe` (Schannel, the Windows certificate store), key passed on stdin | No HTTP library in the exe, key never on a command line |
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
  Rust uses curl's `--ssl-revoke-best-effort`, which skips the check only when the
  list can't be reached and still rejects revoked certificates.
- **Serve direction.** Java uses `Math.random()`; Rust uses the parity of
  `GetTickCount64()`. Both are 50/50.
- **Requirements.** Rust needs Windows 10 1803 or later (for `curl.exe` and the
  high-resolution timer; older Windows falls back to a plain timer and no aiming).
