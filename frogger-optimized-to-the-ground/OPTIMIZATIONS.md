# Java `frogger` vs Rust `frogger-rs.exe`

Same game, same rules, same PNG assets. Java ships a JVM plus an OpenGL layer;
Rust calls Windows directly and rasters the dirty pixels itself.

## Benchmark

i5-6500, HD 530, 1920×1080, Windows 11. Java runs with Temurin 21 (`java -cp
"lib\*;." frogger`, P2D/JOGL). Startup is window-title poll, 3 warm runs each;
CPU is a 10 s `TotalProcessorTime` sample taken ~5 s in, while the game runs.
The Java renderer is GPU-accelerated (OpenGL), which keeps its process quiet but
holds only ~35 FPS at this window size; the Rust build is a CPU rasteriser that
holds 60.

| | Java | Rust | |
|---|---|---|---|
| Program on disk | 14.2 MB game folder (22 files, 7.2 MB of jars) + 328 MB JDK (490 files) | 30 KB exe | ~480× / >10,000× |
| Assets | 6.9 MB of PNGs (11) | same 6.9 MB | |
| Startup | 2.7–4.3 s | 0.70–0.75 s | ~4–6× |
| CPU (% of one core) | 8–14 % | 15–18 % | GPU vs CPU note |
| FPS while measured | 35/60 | 60/60 | |
| Working set | 307.9 MB | 26.2 MB | ~12× |
| Private memory | 280.7 MB | 19.5 MB | ~14× |
| Processes / threads | 1 / 36 | 1 / 4 | |
| Handles | 585 | 111 | |

The CPU row is the honest trade of this rewrite: Java's P2D pipeline composes
the full frame on the GPU (and still misses its own 60 Hz target by half), while
this build composites only the changed rectangles on the CPU and keeps a locked
60. Per frame the dirty area is roughly a quarter of the window (~6 vehicles ×
420×140 and ~5 logs × up to 293×112, before+after boxes), so scalar fill + blit
+ `BitBlt` costs ~2–3 ms. A GPU could do the same pixels cheaper; the point of
this build is that nothing outside the system libraries is needed to do them at
60 Hz.

## Why

- No runtime: `no_std`, no CRT, no crates — imports only `user32`, `gdi32`,
  `kernel32`. All 7.2 MB of Processing/JOGL/kotlin jars and the 328 MB JDK drop
  out entirely.
- PNG decode in-process: hand-written zlib inflate + PNG unfilter over one
  `VirtualAlloc` scratch, straight into fixed buffers. Vehicles are flattened to
  the road colour and logs to the water colour at load, Steve keeps his alpha —
  exactly what Processing's `image()` blended on the GPU.
- Redraws only what moved: entities push before/after rectangles, touching ones
  merge, each merged rect composes and `BitBlt`s once. Java repaints the full
  window and swaps a GL surface every frame.
- One high-res waitable timer (`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`,
  Windows 10 1803+) paced by `QueryPerformanceCounter` instead of Processing's
  frame loop, so it holds a steady 60 FPS where Java lands at ~35.
- No allocations and no GC: fixed static buffers everywhere (slot arrays mirror
  Java's preallocated `Vehicle[]`/`Log[]`, freed slots are reused), zero heap,
  no background compiler threads — hence 4 threads instead of 36.
- Game logic is Java's exact integer math in the exact call order (spawn gates,
  jump arc, gap predictions; the port's xorshift64* lands every draw in Java's
  exact ranges and call order), locked down by unit tests, so the feel of the
  game is unchanged at 60 FPS.

## Differences

- Needs Windows 10 1803+ for the high-resolution timer; on older systems the
  pacer falls back to whole-millisecond waits.
- Built with a standalone llvm-mingw `x86_64-pc-windows-gnullvm` toolchain (no
  rustup, no MSVC); `build.ps1` drives it. Unit tests link the normal import
  libraries instead of raw DLL imports.
- Assets are read from `assets\` next to the exe: byte-for-byte the same PNGs
  the Java game loads.
