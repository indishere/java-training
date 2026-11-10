// Breakout (Rust): a rewrite of `../breakout` (Game.java, Ball.java, Paddle.java,
// Brick.java, BrickGrid.java), optimized to the ground like ../pong-optimized-to-the-ground.
// No Rust standard library and no C runtime: the exe imports only from DLLs that ship with
// Windows (raw-dylib, so no import libraries or Visual Studio are needed).
// `cargo test` builds with std and ordinary import libraries.
#![cfg_attr(not(test), no_std)] // release build: no Rust standard library
#![cfg_attr(not(test), no_main)] // release build: no implicit main(); we provide WinMainCRTStartup
#![windows_subsystem = "windows"] // GUI subsystem: Windows starts it without a console
#![cfg_attr(test, allow(dead_code))] // tests also use helpers the exe path never calls

use core::cell::UnsafeCell; // raw interior mutability for the global game state
use core::ffi::c_void; // untyped pointer target — what Win32 handles are
use core::ptr::{null, null_mut}; // null pointers without the std lib

const FPS: i32 = 60; // the fixed frame rate (Game.java's 60 FPS timer)
const MAX_LEVEL: i32 = 5; // BrickGrid.java: five levels, then the win screen

// Game.java / Paddle.java constants.
const PADDLE_WIDTH: i32 = 110;
const PADDLE_HEIGHT: i32 = 14;
const PADDLE_SPEED: f64 = 9.0;   // Paddle.java KEY_SPEED
const PADDLE_EDGE: f64 = 8.0;    // Paddle.java EDGE_GAP
const PADDLE_BOTTOM: i32 = 42;   // Game.java: paddle top y = height - 42
const BALL_RADIUS: i32 = 16;     // Ball.java radius; drawn 2r x 2r

// BrickGrid.java constants.
const BRICK_COLUMNS: usize = 10; // BrickGrid.java CELL_WIDTH
const BRICK_ROWS: usize = 5; // BrickGrid.java CELL_HEIGHT
const BRICK_CELLS: usize = BRICK_ROWS * BRICK_COLUMNS; // the wall flattened to 50 cells
const BRICK_SIDE: i32 = 50; // BrickGrid.java SIDE: left/right wall margin
const BRICK_TOP: i32 = 70; // BrickGrid.java TOP: y where the wall starts
const BRICK_GAP: i32 = 5; // BrickGrid.java GAP: space between bricks
const BRICK_HEIGHT: i32 = 22; // Brick.java HEIGHT
const LIVES: i32 = 3; // Game.java: three lives, drawn as hearts
const LEVEL_CLEAR_FRAMES: i32 = 90; // Game.java: "LEVEL CLEARED" lasts 1.5 s

// Ball.java's angles in radians: launch (35 degrees) and the paddle bounce half-width (60).
const LAUNCH_TD: f64 = 0.6108652381980153; // Math.toRadians(35)
const BOUNCE_TD: f64 = 1.0471975511965976; // Math.toRadians(60)

// ---- Win32: only the calls this game makes, declared by hand so no crates are needed ----

type H = *mut c_void; // every Win32 handle is this untyped pointer

#[repr(C)] // field order must match Win32's RECT exactly
#[derive(Clone, Copy)] // Rect is passed around by value
struct Rect {
    left: i32,   // x of the left edge
    top: i32,    // y of the top edge
    right: i32,  // x of the right edge (exclusive)
    bottom: i32, // y of the bottom edge (exclusive)
}

#[repr(C)] // field order must match Win32's WNDCLASS exactly
struct WndClass {
    style: u32,                    // class styles (CS_*); unused here
    wnd_proc: unsafe extern "system" fn(H, u32, usize, isize) -> isize, // the window's message pump
    cls_extra: i32,                // extra class bytes; unused
    wnd_extra: i32,                // extra window bytes; unused
    instance: H,                   // the exe's module handle
    icon: H,                       // window icon; none
    cursor: H,                     // the cursor shown over the window
    background: H,                 // brush that clears the window background
    menu_name: *const u16,         // class menu; none
    class_name: *const u16,        // the class's name
}

#[repr(C)] // field order must match Win32's MSG exactly
struct Msg {
    hwnd: H,           // window the message is for
    message: u32,      // the message id (WM_*)
    wparam: usize,     // first parameter (key code, etc.)
    lparam: isize,     // second parameter (mouse coords, size)
    time: u32,         // when it was posted
    pt: [i32; 2],      // cursor position at post time
    private: u32,      // extra field Windows may fill
}

#[repr(C)] // field order must match Win32's PAINTSTRUCT exactly
struct PaintStruct {
    hdc: H,              // device context clipped to the dirty area
    erase: i32,          // whether the background should be erased
    paint: Rect,         // the bounding rect of what needs painting
    restore: i32,        // internal (reserved)
    inc_update: i32,     // internal (reserved)
    reserved: [u8; 32],  // internal (reserved)
}

#[repr(C)] // field order must match Win32's MINMAXINFO exactly
struct MinMaxInfo {
    reserved: [i32; 2],  // internal
    max_size: [i32; 2],  // maximized size limit
    max_pos: [i32; 2],   // maximized position limit
    min_track: [i32; 2], // minimum tracking size (Java: window.setMinimumSize(640, 480))
    max_track: [i32; 2], // maximum tracking size limit
}

#[cfg_attr(all(windows, not(test)), link(name = "user32", kind = "raw-dylib"))] // user32.dll, resolved statically from its exports
#[cfg_attr(all(windows, test), link(name = "user32"))] // tests use the ordinary import library
unsafe extern "system" {
    fn RegisterClassW(class: *const WndClass) -> u16; // registers the window class
    fn CreateWindowExW(ex_style: u32, class: *const u16, title: *const u16, style: u32,
                       x: i32, y: i32, w: i32, h: i32,
                       parent: H, menu: H, instance: H, param: H) -> H; // creates the window
    fn DefWindowProcW(hwnd: H, msg: u32, wparam: usize, lparam: isize) -> isize; // default handling we skip
    fn ShowWindow(hwnd: H, cmd: i32) -> i32; // shows/hides a window
    fn UpdateWindow(hwnd: H) -> i32; // forces an immediate paint of the window
    fn PeekMessageW(msg: *mut Msg, hwnd: H, min: u32, max: u32, remove: u32) -> i32; // polls the message queue without blocking
    fn DispatchMessageW(msg: *const Msg) -> isize; // routes a message to its window proc
    fn PostQuitMessage(code: i32); // queues the quit message
    fn MsgWaitForMultipleObjects(count: u32, handles: *const H, wait_all: i32, ms: u32, wake: u32) -> u32; // sleeps until the timer fires or input arrives
    fn InvalidateRect(hwnd: H, rect: *const Rect, erase: i32) -> i32; // marks a rectangle dirty for the next paint
    fn GetDC(hwnd: H) -> H; // borrows a device context from a window
    fn ReleaseDC(hwnd: H, dc: H) -> i32; // returns a borrowed device context
    fn BeginPaint(hwnd: H, ps: *mut PaintStruct) -> H; // gets a paint DC for the dirty area
    fn EndPaint(hwnd: H, ps: *const PaintStruct) -> i32; // releases the paint DC
    fn GetUpdateRgn(hwnd: H, rgn: H, erase: i32) -> i32; // fetches the dirty region as a GDI shape
    fn FillRect(dc: H, rect: *const Rect, brush: H) -> i32; // paints a rectangle with the given brush
    fn LoadCursorW(instance: H, name: *const u16) -> H; // loads a cursor shape
    fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex_style: u32) -> i32; // converts a client size into a window size
    fn GetSystemMetrics(index: i32) -> i32; // reads system values (screen size, etc.)
}

#[cfg_attr(all(windows, not(test)), link(name = "gdi32", kind = "raw-dylib"))] // gdi32.dll, resolved statically from its exports
#[cfg_attr(all(windows, test), link(name = "gdi32"))] // tests use the ordinary import library
unsafe extern "system" {
    fn GetStockObject(index: i32) -> H; // built-in pen/brush by stock id
    fn SetDCBrushColor(dc: H, color: u32) -> u32; // sets the DC brush colour
    fn SetDCPenColor(dc: H, color: u32) -> u32; // sets the DC pen colour
    fn SelectObject(dc: H, object: H) -> H; // installs an object into the DC, returning the old
    fn Ellipse(dc: H, left: i32, top: i32, right: i32, bottom: i32) -> i32; // draws a filled ellipse
    fn Rectangle(dc: H, left: i32, top: i32, right: i32, bottom: i32) -> i32; // draws a border+fill rectangle
    fn SetTextColor(dc: H, color: u32) -> u32; // sets the text colour
    fn SetBkMode(dc: H, mode: i32) -> i32; // transparent behind-glyph pixels
    fn SetTextAlign(dc: H, align: u32) -> u32; // centred text, positioned at the baseline
    fn TextOutW(dc: H, x: i32, y: i32, text: *const u16, len: i32) -> i32; // draws a UTF-16 string
    fn CreateFontW(height: i32, width: i32, escapement: i32, orientation: i32, weight: i32,
                   italic: u32, underline: u32, strike_out: u32, charset: u32,
                   out_precision: u32, clip_precision: u32, quality: u32,
                   pitch_and_family: u32, face: *const u16) -> H; // creates a logical font
    fn DeleteObject(object: H) -> i32; // frees a GDI object
    fn CreateCompatibleDC(dc: H) -> H; // memory DC for the back buffer
    fn CreateCompatibleBitmap(dc: H, w: i32, h: i32) -> H; // bitmap matching the screen's format
    fn DeleteDC(dc: H) -> i32; // frees a memory DC
    fn BitBlt(dst: H, x: i32, y: i32, w: i32, h: i32, src: H, sx: i32, sy: i32, rop: u32) -> i32; // copies pixels between DCs
    fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> H; // a GDI region shape
    fn SelectClipRgn(dc: H, rgn: H) -> i32; // clips all painting to that region
}

#[cfg_attr(all(windows, not(test)), link(name = "kernel32", kind = "raw-dylib"))] // kernel32.dll, resolved statically from its exports
#[cfg_attr(all(windows, test), link(name = "kernel32"))] // tests use the ordinary import library
unsafe extern "system" {
    fn GetTickCount64() -> u64; // ms since boot (the launch coin flip)
    fn GetModuleHandleW(name: *const u16) -> H; // the exe's module/instance handle
    fn CreateWaitableTimerExW(attributes: *const c_void, name: *const u16, flags: u32, access: u32) -> H; // high-resolution timer
    fn SetWaitableTimer(timer: H, due: *const i64, period: i32,
                        completion: *const c_void, arg: *const c_void, resume: i32) -> i32; // arms the timer
    fn QueryPerformanceCounter(count: *mut i64) -> i32; // the frame clock's current tick
    fn QueryPerformanceFrequency(freq: *mut i64) -> i32; // the clock's ticks per second
    fn ExitProcess(code: u32) -> !; // ends the whole process immediately
}

// The compiler may emit calls to these for struct copies and zeroing (it depends on the
// opt-level). ntdll is mapped into every process anyway, and only what's used gets imported.
#[cfg(all(windows, not(test)))] // only needed for the no_std exe, not the tests
#[link(name = "ntdll", kind = "raw-dylib")] // ntdll.dll, mapped in every process
#[allow(dead_code)] // only what the optimizer emits actually gets imported
unsafe extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void; // copies memory
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void; // copies overlapping memory
    fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void; // fills memory
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32; // compares memory
}

const WM_DESTROY: u32 = 0x0002; // window is being destroyed
const WM_SIZE: u32 = 0x0005; // the client size changed
const WM_KILLFOCUS: u32 = 0x0008; // the window lost keyboard focus
const WM_PAINT: u32 = 0x000F; // something needs repainting
const WM_QUIT: u32 = 0x0012; // PostQuitMessage was called
const WM_ERASEBKGND: u32 = 0x0014; // background erase request; we answer "done"
const WM_GETMINMAXINFO: u32 = 0x0024; // asks for our min/max window sizes
const WM_KEYDOWN: u32 = 0x0100; // a key was pressed
const WM_KEYUP: u32 = 0x0101; // a key was released
const WM_MOUSEMOVE: u32 = 0x0200; // the mouse moved over the window
const VK_LEFT: usize = 0x25; // arrow-left key code
const VK_RIGHT: usize = 0x27; // arrow-right key code
const VK_SPACE: usize = 0x20; // space key code
const VK_RETURN: usize = 0x0D; // enter key code
const VK_A: usize = 0x41; // Java also binds A/D to the paddle
const VK_D: usize = 0x44; // the D key code
const VK_P: usize = 0x50; // the P key code
const VK_R: usize = 0x52; // the R key code
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000; // title bar + border + resize controls
const SW_SHOWNORMAL: i32 = 1; // ShowWindow: show at the normal size
const PM_REMOVE: u32 = 1; // PeekMessageW: take the message out of the queue
const QS_ALLINPUT: u32 = 0x04FF; // wake on any input (mouse, keys, ...)
const INFINITE: u32 = 0xFFFF_FFFF; // wait forever if nothing arrives
const BLACK_BRUSH: i32 = 4; // stock pen/brush id for GetStockObject()
const DC_BRUSH: i32 = 18; // stock pen/brush id for GetStockObject()
const DC_PEN: i32 = 19; // stock pen/brush id for GetStockObject()
const TRANSPARENT: i32 = 1; // SetBkMode: leave the background behind glyphs alone
const TA_CENTER: u32 = 6; // SetTextAlign flag: centre horizontally
const TA_BASELINE: u32 = 24; // SetTextAlign flag: y points at the baseline
const SRCCOPY: u32 = 0x00CC_0020; // BitBlt mode: plain pixel copy
const NULLREGION: i32 = 1; // GetUpdateRgn: nothing is dirty
const IDC_ARROW: usize = 32512; // stock id: the standard arrow cursor
const FW_BOLD: i32 = 700; // CreateFontW weight: bold (Java's "SansSerif" bold)
const DEFAULT_CHARSET: u32 = 1; // CreateFontW: the system's default charset
const CLEARTYPE_QUALITY: u32 = 5; // CreateFontW: use subpixel text smoothing
const CREATE_WAITABLE_TIMER_HIGH_RESOLUTION: u32 = 0x0000_0002; // timer flag: sub-ms accuracy
const TIMER_ALL_ACCESS: u32 = 0x001F_0003; // timer's full access rights
const SM_CXSCREEN: i32 = 0; // GetSystemMetrics: screen width
const SM_CYSCREEN: i32 = 1; // GetSystemMetrics: screen height

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16 // packs the 0x00BBGGRR layout COLORREF wants
}
// The wall paints as one continuous surface: a light-blue gradient from the
// top row (lightest) to the bottom row (deepest). Java had one flat
// Color.BLUE per brick; this keeps a light blue and varies only the brightness.
const BRICK_TOP_COLOR: (u32, u32, u32) = (170, 215, 255); // very light blue, the top row
const BRICK_BOTTOM_COLOR: (u32, u32, u32) = (85, 155, 250); // deeper light blue, the bottom row
// The lives hearts are stamped from the Minecraft hardcore-heart sprite the
// player supplied, so their palette is the sprite's:
const FULL_HEART: u32 = rgb(255, 0, 0);     // the sprite's pure red #FF0000
const SHADING_COLOR: u32 = rgb(77, 0, 0);   // the sprite's dark maroon shading lobes
const DEAD_HEART: u32 = rgb(60, 60, 60);    // a lost life: same shape, greyed out
const DEAD_SHADING: u32 = rgb(30, 30, 30);  // the shading area of a lost heart

// This row's band of the one wall gradient: t is 0 at the top of the surface,
// 1 at its bottom, and every channel fades linearly between the two ends.
fn brick_color(row: usize) -> u32 {
    let t = row as f64 / (BRICK_ROWS - 1) as f64; // 0, 0.25, 0.5, 0.75, 1 across the 5 rows
    let (tr, tg, tb) = BRICK_TOP_COLOR; // the light end of the ramp
    let (br, bg, bb) = BRICK_BOTTOM_COLOR; // the deep end of the ramp
    rgb(
        (tr as f64 + (br as f64 - tr as f64) * t) as u32, // the red channel (small anyway)
        (tg as f64 + (bg as f64 - tg as f64) * t) as u32, // the green channel
        (tb as f64 + (bb as f64 - tb as f64) * t) as u32, // the blue channel, the dominant one
    )
}

// ASCII string literal -> UTF-16 array, at compile time. `wz!` adds the terminating 0.
macro_rules! w {
    ($s:expr) => {{ // takes a &str literal expression
        const S: &str = $s; // binds it so S.len() is a compile-time constant
        const W: [u16; S.len()] = { // the UTF-16 code-unit array
            let (b, mut out, mut i) = (S.as_bytes(), [0u16; S.len()], 0); // ascii bytes, output, cursor
            while i < b.len() { // every ASCII byte widens to exactly one u16
                out[i] = b[i] as u16; // zero-extend the byte
                i += 1;
            }
            out
        };
        &W // hand out a reference into the exe's read-only data
    }};
}
macro_rules! wz {
    ($s:literal) => { w!(concat!($s, "\0")).as_ptr() }; // the same plus the NUL terminator, as a *const u16
}

// Shared mutable state. The game is only touched on the window thread.
struct Global<T>(UnsafeCell<T>); // UnsafeCell wrapped only to add an unsafe Sync impl
unsafe impl<T> Sync for Global<T> {} // legal here: exactly one thread ever touches the state
impl<T> Global<T> {
    #[allow(clippy::mut_from_ref)] // fine: single-threaded, like Java's fields
    fn get(&self) -> &mut T { // hands out a unique reference to the state
        unsafe { &mut *self.0.get() } // turns the raw cell pointer into &mut T
    }
}

// Fixed-size append-only buffer; anything past the end is dropped (callers check `full`).
struct Buf<T: Copy + From<u8>, const N: usize> {
    data: [T; N], // fixed-size backing storage, no heap
    len: usize,   // how many of those N items are used
}

impl<T: Copy + From<u8>, const N: usize> Buf<T, N> {
    const fn new(zero: T) -> Self { // a zeroed, empty buffer (const so statics can hold one)
        Buf { data: [zero; N], len: 0 }
    }
    fn push_ascii(&mut self, text: &[u8]) { // widen ASCII bytes into T (u16 for TextOutW)
        for &b in text {
            if self.len < N { // drop anything past the fixed size
                self.data[self.len] = T::from(b); // zero-extend into the target type
                self.len += 1;
            }
        }
    }
    fn push_int(&mut self, n: i32) { // decimal digits, written by hand (no std formatting)
        let (mut digits, mut i, mut v) = ([0u8; 11], 11, n.unsigned_abs()); // scratch, cursor, value
        loop {
            i -= 1; // fill the digit buffer backwards
            digits[i] = b'0' + (v % 10) as u8; // the lowest digit, as ASCII
            v /= 10; // drop that digit
            if v == 0 {
                break; // ran out of digits
            }
        }
        if n < 0 {
            self.push_ascii(b"-"); // a leading minus for negatives
        }
        self.push_ascii(&digits[i..]); // all digits we produced
    }
}

// Tests read a message line back as text; the buffer is ASCII-only by construction.
#[cfg(test)] // test-only helper: the exe never needs std's String
impl<const N: usize> Buf<u16, N> {
    fn as_text(&self) -> String { // collects the stored u16s back into a std string
        self.data[..self.len].iter().map(|&w| w.to_le_bytes()[0] as char).collect()
    }
}

// core has no float rounding without libm, and these only need whole pixels.
fn floor(x: f64) -> f64 {
    let t = x as i64 as f64; // the value truncated toward zero
    if t > x { t - 1.0 } else { t } // went past it downwards? step back one
}

fn ceil(x: f64) -> f64 {
    let t = x as i64 as f64; // the value truncated toward zero
    if t < x { t + 1.0 } else { t } // fell short? step up one
}

fn abs(x: f64) -> f64 {
    if x < 0.0 { -x } else { x } // flip the sign only when negative
}

// ---- Math.sin / Math.cos ----

// Java needs a real sin/cos: the paddle bounce angle is `Math.toRadians(60) * offset`,
// continuous in the hit position. The ball only ever sees |angle| <= 60 degrees, so a
// plain power series (no libm range reduction) is double-precision accurate: the next
// unused term for either series is below 1e-13 over the whole interval.
fn sin(x: f64) -> f64 { // the odd Taylor series, 8 terms cover |x| <= 60 degrees
    let x2 = x * x; // x^2, reused by every term
    let (mut term, mut sum) = (x, x); // the first term IS x; that's also the running total
    let mut n = 1; // term index
    while n <= 7 { // terms x, -x^3/3!, ..., down to -x^15/15!
        term *= -x2 / (2.0 * n as f64 * (2.0 * n as f64 + 1.0)); // build each term from the last
        sum += term; // add it to the total
        n += 1;
    }
    sum
}

fn cos(x: f64) -> f64 { // the even Taylor series, 9 terms
    let x2 = x * x; // x^2, reused by every term
    let (mut term, mut sum) = (1.0, 1.0); // the first term is 1
    let mut n = 1; // term index
    while n <= 8 { // terms 1, -x^2/2!, ..., down to -x^16/16!
        term *= -x2 / (2.0 * n as f64 - 1.0) / (2.0 * n as f64); // build each term from the last
        sum += term; // add it to the total
        n += 1;
    }
    sum
}

// ---- Game ----

// Game.java's enum State.
#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    Ready, Playing, Paused, LevelClear, GameOver, Won, // one variant per Java state
}

// BrickGrid.java's LEVELS (1 = brick, 0 = empty), one [row][column] grid per level.
const LEVELS: [[[u8; BRICK_COLUMNS]; BRICK_ROWS]; 5] = [
    [ // level 1: the full 5x10 wall, every row solid
        *b"1111111111", // a solid row of ten bricks
        *b"1111111111",
        *b"1111111111",
        *b"1111111111",
        *b"1111111111",
    ],
    [ // level 2: a checkerboard, odd/even columns swap every row
        *b"1010101010", // bricks on even columns
        *b"0101010101", // bricks on odd columns
        *b"1010101010",
        *b"0101010101",
        *b"1010101010",
    ],
    [ // level 3: a pyramid, one more brick per row outwards
        *b"0000110000", // two bricks, dead centre
        *b"0001111000",
        *b"0011111100",
        *b"0111111110",
        *b"1111111111", // the full base row
    ],
    [ // level 4: a solid border around a hollow core with a brick island
        *b"1111111111", // the top edge
        *b"1000000001", // hollow except the corners
        *b"1001111001", // corners plus the centre island
        *b"1000000001",
        *b"1111111111", // the bottom edge
    ],
    [ // level 5: a diamond, widest at the middle row
        *b"0000110000", // the two-centre top
        *b"0001111000",
        *b"0011111100", // the widest row
        *b"0001111000",
        *b"0000110000", // back to two bricks
    ],
];

// Ball.java: positions and speeds are doubles, (x, y) is the middle of the ball.
#[derive(Clone, Copy, PartialEq)]
struct Ball {
    x: f64,      // centre x in pixels
    y: f64,      // centre y in pixels
    speed_x: f64, // horizontal pixels per tick
    speed_y: f64, // vertical pixels per tick
}

// One line of a state overlay (Game.java drawMessage): baseline offset from the vertical
// centre, a font index into FONT_SIZES, and the UTF-16 text Java concatenates at runtime.
struct Message {
    delta: i32,       // baseline y offset from the vertical centre
    font: usize,      // index into self.fonts / FONT_SIZES
    buf: Buf<u16, 32>, // the line's text, as UTF-16 for TextOutW
}

// Java's bold "SansSerif" sizes: 48 / 42 / 20 / 18. Indexes into FONTS.
const FONT_SIZES: [i32; 4] = [48, 42, 20, 18];

struct Game {
    window: H,       // the Win32 window handle
    width: i32,      // Game.width tracks the JPanel's size (client area)
    height: i32,     // client-area height in pixels
    state: State,    // which of Game.java's screens we are on
    state_timer: i32, // Game.stateTimer (only LEVEL_CLEAR counts it down)
    score: i32,      // Brick points plus level-clear bonuses
    lives: i32,      // hearts still alive
    level: i32,      // 1..=MAX_LEVEL
    bricks_left: i32, // Java rescans the wall for isCleared(); counting gives it in O(1)

    // Paddle.java. x is the paddle's middle (double); the top y is an int.
    paddle_x: f64,      // the paddle's centre, as Java keeps it
    paddle_y: i32,      // the paddle's top, width-derived
    moving_left: bool,  // Paddle.moveLeft(true) state
    moving_right: bool, // Paddle.moveRight(true) state
    mouse_target: f64,  // -1 = none; the mouse target wins until any movement key is pressed

    ball: Ball,       // the one ball
    ball_speed: f64,  // Ball.java's `speed`, set at every level start

    // BrickGrid.java as a flat row-major wall: cell bounds derive from brick_width, so a
    // resize/reflow is one division instead of 50 Rectangle writes.
    brick_alive: [bool; BRICK_CELLS], // row-major alive flags
    brick_width: i32,                 // computed column width, Java's reflow

    fonts: [H; FONT_SIZES.len()], // the four overlay fonts, made once
    back_dc: H,                   // memory DC holding the back bitmap
    back_bitmap: H,               // the off-screen image we paint into
    original_bitmap: H,           // the DC's placeholder bitmap, kept for release
    dirty_region: H,              // GDI region: what needs repainting this frame
    msg: [Message; 3],            // the up-to-three overlay lines
}

static GAME: Global<Game> = Global(UnsafeCell::new(Game::new())); // the one global instance

fn game() -> &'static mut Game { // the pointer Java calls `this`
    GAME.get()
}

impl Ball {
    // Java Ball.reset: sits still on the launch pad.
    fn reset(&mut self, x: f64, y: f64) {
        *self = Ball { x, y, speed_x: 0.0, speed_y: 0.0 }; // overwrite ball with a still one
    }

    // Java Ball.stickTo(paddle): resting 1 px above the paddle's face.
    fn stick_to(&mut self, paddle_x: f64, paddle_y: i32) {
        self.x = paddle_x; // centred on the paddle
        self.y = (paddle_y - BALL_RADIUS - 1) as f64; // wedged 1 px above its top
    }
}

impl Game {
    const fn new() -> Self { // the const defaults Java's fields start with
        Game {
            window: null_mut(),  // no window yet
            width: 800,          // Java frame.pack() on the 800x600 JPanel
            height: 600,
            state: State::Ready, // start on the "press space" screen
            state_timer: 0,      // unused outside LEVEL_CLEAR
            score: 0,            // a fresh game scores zero
            lives: LIVES,        // three hearts
            level: 1,            // at level 1
            bricks_left: 0,      // start_level(1) fills this in
            paddle_x: 400.0,     // the middle of the 800-wide default panel
            paddle_y: 600 - PADDLE_BOTTOM, // y 558 at the default height
            moving_left: false,  // no keys/mouse until input arrives
            moving_right: false,
            mouse_target: -1.0,  // -1 means "no mouse target"
            ball: Ball { x: 0.0, y: 0.0, speed_x: 0.0, speed_y: 0.0 }, // reset()/start_level() places it
            ball_speed: 5.9,     // level-1 speed, recomputed at every start_level
            brick_alive: [false; BRICK_CELLS], // start_level() re-fills the wall
            brick_width: 65,     // (800 - 100 - 45) / 10 at the default width
            fonts: [null_mut(); FONT_SIZES.len()], // run() creates the fonts
            back_dc: null_mut(), // run/WM_SIZE create the back buffer
            back_bitmap: null_mut(),
            original_bitmap: null_mut(),
            dirty_region: null_mut(),
            msg: [ // the three overlay lines' geometry, matching drawMessage
                Message { delta: -55, font: 3, buf: Buf::new(0) }, // above centre, small font
                Message { delta: 0, font: 0, buf: Buf::new(0) },   // on centre, the big heading
                Message { delta: 40, font: 3, buf: Buf::new(0) },  // under centre, small font
            ],
        }
    }

    // Game.resetGame: brand new game, level 1.
    fn reset(&mut self) {
        self.score = 0; // wipe the score
        self.lives = LIVES; // all hearts back
        self.paddle_x = (self.width / 2) as f64; // paddle centred
        self.start_level(1); // from the top: level 1
    }

    // Game.startLevel: fresh wall from the layout table, faster ball, launch pad.
    fn start_level(&mut self, new_level: i32) {
        self.level = new_level; // remember which layouts/speed to use
        let layout = LEVELS[(new_level - 1) as usize]; // this level's row-major bitmap
        self.bricks_left = 0; // recount from zero (Java rescans instead)
        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLUMNS {
                let alive = layout[row][col] == b'1'; // '1' means a brick
                self.brick_alive[row * BRICK_COLUMNS + col] = alive; // store into the flat wall
                self.bricks_left += alive as i32; // count true as 1
            }
        }
        self.reflow(); // derive brick widths for the new wall
        self.ball_speed = 5.6 + new_level as f64 * 0.3; // Game.startLevel's setSpeed
        self.ball.reset(self.paddle_x, (self.paddle_y - BALL_RADIUS - 1) as f64);
        self.paddle_x = (self.width / 2) as f64; // paddle re-centred for the next level
        self.state = State::Ready; // wait on the launch pad
        self.build_messages(); // the "LEVEL n / BREAKOUT / press space" overlay
    }

    // BrickGrid.reflow: only the column width changes with the window size.
    fn reflow(&mut self) {
        // *one* division gives every brick's width (Java rewrote 50 Rectangles)
        self.brick_width = (self.width - BRICK_SIDE * 2 - BRICK_GAP * (BRICK_COLUMNS as i32 - 1))
            / BRICK_COLUMNS as i32;
    }

    fn brick_x(&self, col: usize) -> i32 { // a column's left edge x
        BRICK_SIDE + col as i32 * (self.brick_width + BRICK_GAP) // wall margin then pitch per column
    }

    fn brick_y(&self, row: usize) -> i32 { // a row's top edge y
        BRICK_TOP + row as i32 * (BRICK_HEIGHT + BRICK_GAP) // top offset then pitch per row
    }

    // Game.updateGame: once per frame. Returns true when the state changed and the whole
    // window needs a repaint; partial sprite moves are invalidated in place.
    fn update(&mut self) -> bool {
        let (old_ball, old_paddle_x, old_state) = (self.ball, self.paddle_x, self.state); // frame-start snapshot for dirty rects

        // Java calls paddle.update(width) every tick: the keys move it even while paused,
        // and only the mouse is gated to READY/PLAYING (Game.mouseMoved).
        self.paddle_update();

        match self.state {
            // READY: the ball rides the paddle until SPACE launches it.
            State::Ready => self.ball.stick_to(self.paddle_x, self.paddle_y),
            State::Playing => self.playing(), // the PLAYING branch, Ball.update et al
            _ => {} // PAUSED/LEVEL_CLEAR/GAME_OVER/WON tick nothing
        }

        // Java runs LEVEL_CLEAR's countdown at the end of the same updateGame() that set
        // it, so a freshly cleared wall already decrements this frame too.
        if self.state == State::LevelClear {
            self.state_timer -= 1; // one tick of the 90-frame countdown
            if self.state_timer <= 0 { // the "LEVEL CLEARED" screen is over
                self.score += 100 * self.level; // the clear bonus (100 * level)
                if self.level >= MAX_LEVEL {
                    self.state = State::Won; // last level done -> the win screen
                    self.build_messages();
                } else {
                    self.start_level(self.level + 1); // next level
                }
            }
        }

        // Overlay screens don't animate: Java repaints them 60x/s, here they repaint once.
        if self.state == State::Ready || self.state == State::Playing {
            self.invalidate_paddle(old_paddle_x); // repaint where the paddle was and is
            if self.ball != old_ball {
                self.invalidate_ball(old_ball); // erase the old ball spot
                self.invalidate_ball(self.ball); // paint the new one
            }
        }
        self.state != old_state // true: the overlay changed, repaint everything
    }

    // Java's PLAYING branch: move, bounce, break, then lose or clear.
    fn playing(&mut self) {
        // Ball.update()
        self.ball.x += self.ball.speed_x; // advance by the velocity
        self.ball.y += self.ball.speed_y;

        // Ball.bounceOffWalls(0, width, 0): left, right and top.
        let (r, width) = (BALL_RADIUS as f64, self.width as f64); // radius and right wall
        if self.ball.x - r < 0.0 && self.ball.speed_x < 0.0 { // past the left edge, moving left
            self.ball.x = r; // clamp onto the wall
            self.ball.speed_x = -self.ball.speed_x; // flip to moving right
        }
        if self.ball.x + r > width && self.ball.speed_x > 0.0 { // past the right edge, moving right
            self.ball.x = width - r; // clamp onto the wall
            self.ball.speed_x = -self.ball.speed_x; // flip to moving left
        }
        if self.ball.y - r < 0.0 && self.ball.speed_y < 0.0 { // past the top edge, moving up
            self.ball.y = r; // clamp onto the wall
            self.ball.speed_y = -self.ball.speed_y; // flip to falling
        }
        // (no bottom wall: falling past it is how lives are lost)

        self.bounce_off_paddle(); // Ball.bounceOff(paddle)
        self.score += self.collide_bricks(); // BrickGrid.collide, points included

        if self.ball.y - r > self.height as f64 { // Ball.isBelow
            self.lose_life(); // the ball fell out of the play field
        } else if self.bricks_left == 0 { // BrickGrid.isCleared
            self.state = State::LevelClear; // show "LEVEL CLEARED" + bonus
            self.state_timer = LEVEL_CLEAR_FRAMES; // 90 frames = 1.5 s
            self.build_messages();
        }
    }

    // Ball.bounceOff(paddle): where the ball meets the paddle decides the angle.
    fn bounce_off_paddle(&mut self) {
        if self.ball.speed_y <= 0.0 || !self.touch_paddle() {
            return; // rising balls and misses do nothing (Java's guard)
        }
        let offset = ((self.ball.x - self.paddle_x) / (PADDLE_WIDTH as f64 / 2.0)).clamp(-1.0, 1.0); // -1 left edge .. 1 right edge
        let angle = BOUNCE_TD * offset; // the bounce angle, at most 60 degrees
        self.ball.speed_x = sin(angle) * self.ball_speed; // across, per the hit point
        self.ball.speed_y = -cos(angle) * self.ball_speed; // up, keeping Ball.speed constant
        self.ball.y = (self.paddle_y - BALL_RADIUS - 1) as f64; // snap out of the paddle
    }

    // getBounds().intersects(paddle.getBounds()) on Java's truncated int boxes.
    fn touch_paddle(&self) -> bool {
        let (span, half) = (BALL_RADIUS * 2, PADDLE_WIDTH / 2); // the ball box is 2r wide
        let (bx, by) = (self.ball.x as i32 - BALL_RADIUS, self.ball.y as i32 - BALL_RADIUS); // ball box's top-left
        let px = self.paddle_x as i32 - half; // paddle box's left edge
        bx < px + PADDLE_WIDTH && bx + span > px && by < self.paddle_y + PADDLE_HEIGHT && by + span > self.paddle_y
    }

    // Ball.bounceOff(brick) + BrickGrid.collide(): the first live brick the ball touches,
    // walked in the same row-major order Java fgloves its list. Returns the brick's points.
    fn collide_bricks(&mut self) -> i32 {
        let (r, span) = (BALL_RADIUS as f64, BALL_RADIUS * 2); // radius and ball-box width
        let (bx, by) = (self.ball.x as i32 - BALL_RADIUS, self.ball.y as i32 - BALL_RADIUS); // ball box's top-left
        for cell in 0..BRICK_CELLS { // row-major, exactly Java's list order
            if !self.brick_alive[cell] {
                continue; // already broken bricks don't exist
            }
            let (row, col) = (cell / BRICK_COLUMNS, cell % BRICK_COLUMNS); // flatten/unflatten this cell
            let (x, w, y, h) = (self.brick_x(col), self.brick_width, self.brick_y(row), BRICK_HEIGHT); // the brick's box
            if !(bx < x + w && bx + span > x && by < y + h && by + span > y) {
                continue; // no intersection, next brick
            }
            let (xd, yd, wd, hd) = (x as f64, y as f64, w as f64, h as f64); // the box as floats
            let horizontal = ((self.ball.x + r) - xd).min((xd + wd) - (self.ball.x - r)); // how deep we are sideways
            let vertical = ((self.ball.y + r) - yd).min((yd + hd) - (self.ball.y - r)); // how deep we are vertically
            if horizontal < vertical { // Java resolves the smaller penetration; ties go vertical
                if self.ball.x < xd + wd / 2.0 { // we entered from the left half
                    self.ball.x = xd - r; // push out on the left side
                    self.ball.speed_x = -abs(self.ball.speed_x); // bounce left
                } else {
                    self.ball.x = xd + wd + r; // push out on the right side
                    self.ball.speed_x = abs(self.ball.speed_x); // bounce right
                }
            } else {
                if self.ball.y < yd + hd / 2.0 { // we came from above
                    self.ball.y = yd - r; // push out on the top side
                    self.ball.speed_y = -abs(self.ball.speed_y); // bounce up
                } else {
                    self.ball.y = yd + hd + r; // push out below the brick
                    self.ball.speed_y = abs(self.ball.speed_y); // bounce down
                }
            }
            self.brick_alive[cell] = false; // the brick breaks
            self.bricks_left -= 1; // the O(1) isCleared() counter
            // The border Java draws with drawRect reaches 1 px past x + w and y + h.
            let area = Rect { left: x, top: y, right: x + w + 1, bottom: y + h + 1 };
            unsafe { InvalidateRect(self.window, &area, 0) }; // erase exactly this brick
            return (BRICK_ROWS - row) as i32 * 10; // Brick.java points: 50..10 by row
        }
        0 // nothing was hit, no points
    }

    // Paddle.update: the mouse target moves first; keys are 9 px per tick, both clamped.
    fn paddle_update(&mut self) {
        if self.mouse_target >= 0.0 { // a mouse target beats the keys (Game.mouseMoved)
            let distance = self.mouse_target - self.paddle_x; // how far to travel
            if abs(distance) <= PADDLE_SPEED { // close enough to arrive
                self.paddle_x = self.mouse_target; // snap onto the target
            } else {
                self.paddle_x += PADDLE_SPEED * if distance < 0.0 { -1.0 } else { 1.0 }; // step 9 px toward it
            }
        } else {
            if self.moving_left {
                self.paddle_x -= PADDLE_SPEED; // KEY_SPEED per tick left
            }
            if self.moving_right {
                self.paddle_x += PADDLE_SPEED; // KEY_SPEED per tick right
            }
        }
        let (half, edge, width) = (PADDLE_WIDTH as f64 / 2.0, PADDLE_EDGE, self.width as f64); // clamp parameters
        if self.paddle_x - half < edge {
            self.paddle_x = edge + half; // keep the 8 px left edge gap (Paddle.update)
        }
        if self.paddle_x + half > width - edge {
            self.paddle_x = width - edge - half; // and the 8 px right edge gap
        }
    }

    // Game.loseLife: the last life ends the game, otherwise back onto the paddle.
    // Java resets the ball at the paddle's OLD centre first, then centres the paddle.
    fn lose_life(&mut self) {
        self.lives -= 1; // a heart dies
        self.state = if self.lives <= 0 {
            self.build_messages(); // the GAME OVER overlay
            State::GameOver
        } else {
            self.ball.reset(self.paddle_x, (self.paddle_y - BALL_RADIUS - 1) as f64); // still ball at the old paddle centre
            self.paddle_x = (self.width / 2) as f64; // then the paddle centres (Java's order)
            self.build_messages(); // the READY overlay
            State::Ready
        };
    }

    // Space / Enter: Ball.launch() (35 degrees, coin-flip side) or a fresh game.
    // Java's `Math.random() < 0.5 ? -1 : 1`; the coin is given by the caller, usually the
    // system tick counter, so no RNG state is kept.
    fn launch(&mut self, coin: u64) {
        let direction = if coin & 1 == 1 { -1.0 } else { 1.0 }; // the low bit is the coin flip
        self.ball.speed_x = direction * self.ball_speed * sin(LAUNCH_TD); // 35 degrees sideways
        self.ball.speed_y = -self.ball_speed * cos(LAUNCH_TD); // and up, at the level speed
    }

    // Fills the three overlay lines, exactly Game.drawMessage's branches.
    fn build_messages(&mut self) {
        for line in &mut self.msg {
            line.buf.len = 0; // every line starts empty
        }
        match self.state {
            State::Playing => {} // drawMessage writes nothing while playing
            State::Ready => {
                let level = self.level; // the level number goes into line one
                self.write(0, -55, 3, b"LEVEL ");
                self.msg[0].buf.push_int(level); // ... plus its decimal digits
                self.write(1, 0, 0, b"BREAKOUT"); // the big centre heading
                self.write(2, 40, 3, b"PRESS SPACE TO LAUNCH");
            }
            State::Paused => {
                self.write(1, 0, 0, b"PAUSED"); // the centre heading
                self.write(2, 40, 3, b"PRESS P TO CONTINUE");
            }
            State::LevelClear => {
                let level = self.level; // which level just ended
                self.write(0, 0, 1, b"LEVEL ");
                self.msg[0].buf.push_int(level); // ... plus its digits ...
                self.msg[0].buf.push_ascii(b" CLEARED"); // ... and then the word
                self.write(1, 40, 3, b"+");
                self.msg[1].buf.push_int(100 * level); // the bonus number being awarded
                self.msg[1].buf.push_ascii(b" BONUS");
            }
            State::GameOver | State::Won => {
                let heading: &[u8] = if self.state == State::GameOver { b"GAME OVER" } else { b"YOU WIN" }; // the branch only differs in this text
                self.write(0, -20, 0, heading);
                self.write(1, 20, 2, b"SCORE ");
                self.msg[1].buf.push_int(self.score); // the final score digits
                self.write(2, 60, 3, b"PRESS SPACE TO PLAY AGAIN");
            }
        }
    }

    fn write(&mut self, line: usize, delta: i32, font: usize, text: &[u8]) { // one overlay line, Game.writeMessage
        let msg = &mut self.msg[line];
        msg.delta = delta; // where on the vertical centre
        msg.font = font; // which font draws it
        msg.buf.len = 0; // overwrite whatever was there
        msg.buf.push_ascii(text);
    }

    fn invalidate_sprite(&self, x: f64, y: f64, w: i32, h: i32) { // the dirty rect of one sprite box, padded 1 px
        let area = Rect {
            left: floor(x) as i32 - 1, // whole pixels, padded for the AA edge
            top: floor(y) as i32 - 1,
            right: ceil(x + w as f64) as i32 + 1,
            bottom: ceil(y + h as f64) as i32 + 1,
        };
        unsafe { InvalidateRect(self.window, &area, 0) }; // queue just that rectangle
    }

    // Old and new spot of anything that moved; a paddle standing still isn't redrawn.
    fn invalidate_paddle(&self, old_x: f64) {
        if old_x != self.paddle_x { // only motion invalidates (Java repaints either way)
            let y = self.paddle_y as f64; // the paddle never moves vertically
            self.invalidate_sprite(old_x - PADDLE_WIDTH as f64 / 2.0, y, PADDLE_WIDTH, PADDLE_HEIGHT); // erase the old box
            self.invalidate_sprite(self.paddle_x - PADDLE_WIDTH as f64 / 2.0, y, PADDLE_WIDTH, PADDLE_HEIGHT); // paint the new box
        }
    }

    fn invalidate_ball(&self, ball: Ball) { // a ball spot's dirty rect
        self.invalidate_sprite(ball.x - BALL_RADIUS as f64, ball.y - BALL_RADIUS as f64,
                               BALL_RADIUS * 2, BALL_RADIUS * 2);
    }

    // Game.paintComponent's play-state layers: wall, ball, paddle.
    fn paint(&self, dc: H, update: &Rect) {
        unsafe {
            // Java repaints the black background every tick (JPanel's background).
            let full = Rect { left: 0, top: 0, right: self.width, bottom: self.height };
            FillRect(dc, &full, GetStockObject(BLACK_BRUSH));

            // PAUSED/LEVEL_CLEAR/GAME_OVER/WON end with a full-window shade hiding the
            // play field, so those states skip the wall, ball and paddle entirely.
            if self.state == State::Ready || self.state == State::Playing {
                if update.bottom > BRICK_TOP { // the dirty area can't reach the wall at all
                    self.paint_wall(dc);
                }
                self.paint_ball(dc); // the ball layer, only where dirty
                self.paint_paddle(dc); // the paddle layer, only where dirty
            }

            self.paint_message(dc); // the overlay text (empty while playing)
            self.paint_hearts(dc); // the lives row
        }
    }

    // Brick.draw, but the wall shades as ONE surface: each row takes its band of a
    // light-blue gradient (row 0 lightest, row 4 deepest), never per-brick colours.
    fn paint_wall(&self, dc: H) {
        unsafe {
            let pen = GetStockObject(DC_PEN); // the 1 px border pen
            let brush = GetStockObject(DC_BRUSH); // the fill brush, recoloured per row
            let old_pen = SelectObject(dc, pen);
            let old_brush = SelectObject(dc, brush);
            SetDCPenColor(dc, rgb(0, 0, 0)); // Java drawRect's black border
            for row in 0..BRICK_ROWS {
                SetDCBrushColor(dc, brick_color(row)); // this row's band of the one gradient
                for col in 0..BRICK_COLUMNS {
                    if !self.brick_alive[row * BRICK_COLUMNS + col] {
                        continue; // broken bricks leave black
                    }
                    let (x, y) = (self.brick_x(col), self.brick_y(row)); // the cell's top-left
                    // Java drawRect's border lives on the x + w / y + h pixels.
                    Rectangle(dc, x, y, x + self.brick_width + 1, y + BRICK_HEIGHT + 1);
                }
            }
            SelectObject(dc, old_pen); // restore the caller's pen
            SelectObject(dc, old_brush); // and brush
        }
    }

    // Ball.draw: a filled white oval, no pen ring.
    fn paint_ball(&self, dc: H) {
        unsafe {
            let (x, y) = (self.ball.x as i32 - BALL_RADIUS, self.ball.y as i32 - BALL_RADIUS); // the oval box's top-left
            let white = rgb(255, 255, 255); // Java's fill AND pen colour
            SetDCBrushColor(dc, white);
            SetDCPenColor(dc, white); // same colour: Java's drawOval border vanishes into the fill
            let old_pen = SelectObject(dc, GetStockObject(DC_PEN));
            let old_brush = SelectObject(dc, GetStockObject(DC_BRUSH));
            Ellipse(dc, x, y, x + BALL_RADIUS * 2, y + BALL_RADIUS * 2); // the 32x32 white oval
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
        }
    }

    // Paddle.draw: white rectangle at the truncated middle.
    fn paint_paddle(&self, dc: H) {
        unsafe {
            let x = self.paddle_x as i32 - PADDLE_WIDTH / 2; // Java's int cast of the centre
            SetDCBrushColor(dc, rgb(255, 255, 255)); // fill white
            FillRect(dc, &Rect { left: x, top: self.paddle_y, right: x + PADDLE_WIDTH, bottom: self.paddle_y + PADDLE_HEIGHT },
                     GetStockObject(DC_BRUSH)); // draw.fillRect, one call
        }
    }

    // Game.drawMessage: up to three centred lines above/below the vertical centre.
    fn paint_message(&self, dc: H) {
        unsafe {
            let centre = (self.height / 2 - 30).max(360); // drawMessage's `centre`
            SetTextColor(dc, rgb(255, 255, 255)); // Java's white text
            SetBkMode(dc, TRANSPARENT); // don't box the glyphs
            SetTextAlign(dc, TA_CENTER | TA_BASELINE); // drawCentered centres x, baseline at y
            for line in &self.msg { // the three overlay lines
                if line.buf.len == 0 {
                    continue; // an empty line draws nothing
                }
                let old_font = SelectObject(dc, self.fonts[line.font]); // that line's font size
                TextOutW(dc, self.width / 2, centre + line.delta, line.buf.data.as_ptr(), line.buf.len as i32);
                SelectObject(dc, old_font); // back to the caller's font
            }
        }
    }

    // The lives row: each heart is the Minecraft hardcore-heart sprite the player
    // supplied — 13x12 cells at 2 px, one FillRect per horizontal run of one colour.
    fn paint_hearts(&self, dc: H) {
        // The sprite as cell codes: '.' background, 'K' black outline, 'R' body, 'D' shading.
        const HEART_SPRITE: [[u8; 13]; 12] = [
            *b"..KKK...KKK..",
            *b".KRRRK.KRRRK.",
            *b"KRRDDKKKDDRRK",
            *b"KRRDDDKDDDRRK",
            *b"KRRRDDRDDRRRK",
            *b".KRRRRRRRRRK.",
            *b"..KRRRRRRRK..",
            *b"...KRRRRRK...",
            *b"....KRRRK....",
            *b".....KRK.....",
            *b"......K......",
            *b"......K......",
        ];
        unsafe {
            // One stock brush paints every cell; only its colour changes.
            let brush = GetStockObject(DC_BRUSH);
            let old = SelectObject(dc, brush); // remember the caller's brush
            let start_x = self.width - 20 - (3 * 34); // right-aligned row, Java's spacing
            for i in 0..3 { // one heart per life, left to right
                let (full, x) = (i < self.lives, start_x + i as i32 * 34);
                // Full hearts: red body over maroon shading. Lost: two grey tones.
                let (body, shade) = if full {
                    (FULL_HEART, SHADING_COLOR)
                } else {
                    (DEAD_HEART, DEAD_SHADING)
                };
                for (row, cells) in HEART_SPRITE.iter().enumerate() { // the sprite, top down
                    let sy = 18 + row as i32 * 2; // every cell row is 2 px tall
                    let mut col = 0; // the column cursor inside this row
                    while col < cells.len() { // sweep the row left to right
                        let code = cells[col]; // this cell's colour code
                        if code == b'.' { // background: nothing is painted here
                            col += 1;
                            continue;
                        }
                        SetDCBrushColor(dc, match code { // set the brush colour for this run
                            b'R' => body,      // red / grey body cells
                            b'D' => shade,     // maroon / darker grey shading cells
                            _ => rgb(0, 0, 0), // 'K': the black outline
                        });
                        let start = col; // where the current colour run begins
                        while col < cells.len() && cells[col] == code {
                            col += 1; // extend the run while the colour holds
                        }
                        // Paint the whole run as one rectangle spanning its columns.
                        FillRect(dc, &Rect {
                            left: x + start as i32 * 2,
                            top: sy,
                            right: x + col as i32 * 2,
                            bottom: sy + 2,
                        }, brush);
                    }
                }
            }
            SelectObject(dc, old); // restore the caller's brush
        }
    }

    fn release_back_buffer(&mut self) { // the resize path's GDI teardown
        if self.back_dc.is_null() {
            return; // nothing allocated yet
        }
        unsafe {
            if !self.original_bitmap.is_null() {
                SelectObject(self.back_dc, self.original_bitmap); // unhook our bitmap first
            }
            if !self.back_bitmap.is_null() {
                DeleteObject(self.back_bitmap); // free the bitmap memory
            }
            DeleteDC(self.back_dc); // free the memory DC
        }
        self.back_dc = null_mut(); // mark the whole set dead
        self.back_bitmap = null_mut();
        self.original_bitmap = null_mut();
    }

    fn create_back_buffer(&mut self, window: H) { // (re)build the off-screen image
        self.release_back_buffer(); // the old one dies first
        unsafe {
            let screen = GetDC(window); // the screen's pixel format
            self.back_dc = CreateCompatibleDC(screen); // a memory DC matching it
            self.back_bitmap = CreateCompatibleBitmap(screen, self.width, self.height); // the new-sized bitmap
            ReleaseDC(window, screen); // give the screen DC back
            if self.back_dc.is_null() || self.back_bitmap.is_null() {
                self.release_back_buffer(); // allocation failed: stay consistent
                return;
            }
            self.original_bitmap = SelectObject(self.back_dc, self.back_bitmap); // put our bitmap in the DC
        }
    }
}

// Game.updateWindowSize as one WM_SIZE instead of a per-frame poll of the panel size:
// reflow the wall, put the paddle on the new bottom line, stick the waiting ball to it.
unsafe fn on_size(game: &mut Game, window: H, width: i32, height: i32) {
    unsafe {
        game.width = width.max(1); // Java's setSize guards against 0
        game.height = height.max(1);
        game.paddle_y = game.height - PADDLE_BOTTOM; // the paddle rides the new bottom
        game.reflow(); // one division re-sizes every brick column
        if game.state == State::Ready {
            game.ball.stick_to(game.paddle_x, game.paddle_y); // the waiting ball follows the paddle
        }
        game.create_back_buffer(window); // the back buffer must match the new size
        game.build_messages(); // the overlay lines re-space with the new height
        InvalidateRect(window, null(), 0); // everything repaints after a resize
    }
}

unsafe extern "system" fn window_proc(window: H, message: u32, wparam: usize, lparam: isize) -> isize {
    unsafe {
        match message {
            WM_MOUSEMOVE => {
                let game = game();
                if game.state == State::Ready || game.state == State::Playing {
                    // mouseMoved/mouseDragged pass the client-area x (a signed halfword).
                    game.mouse_target = ((lparam & 0xFFFF) as i16) as i32 as f64; // sign-extend, then widen
                }
                0
            }
            WM_KEYDOWN => {
                let game = game();
                match wparam {
                    VK_LEFT | VK_A => { // Paddle.moveLeft(true) also drops the mouse target
                        game.moving_left = true;
                        game.mouse_target = -1.0;
                    }
                    VK_RIGHT | VK_D => {
                        game.moving_right = true;
                        game.mouse_target = -1.0;
                    }
                    VK_SPACE | VK_RETURN => {
                        if game.state == State::Ready {
                            game.launch(GetTickCount64()); // the 35-degree coin-flip launch
                            game.state = State::Playing; // off the launch pad
                            game.build_messages(); // overlay goes empty
                            InvalidateRect(window, null(), 0); // the READY text must go
                        } else if game.state == State::GameOver || game.state == State::Won {
                            game.reset(); // space restarts after the last screen
                            InvalidateRect(window, null(), 0);
                        }
                    }
                    VK_P => {
                        if game.state == State::Playing || game.state == State::Paused {
                            game.state = if game.state == State::Playing { State::Paused } else { State::Playing }; // P toggles
                            game.build_messages();
                            InvalidateRect(window, null(), 0);
                        }
                    }
                    VK_R => {
                        game.reset(); // R restarts from any state
                        InvalidateRect(window, null(), 0);
                    }
                    _ => {} // every other key: ignored
                }
                0
            }
            WM_KEYUP => {
                let game = game();
                // moveLeft(false)/moveRight(false) leave the mouse target alone.
                if wparam == VK_LEFT || wparam == VK_A {
                    game.moving_left = false;
                }
                if wparam == VK_RIGHT || wparam == VK_D {
                    game.moving_right = false;
                }
                0
            }
            WM_KILLFOCUS => {
                // Java can miss a keyReleased while the window loses focus; drop held keys
                // so the paddle doesn't glide off-screen unattended.
                let game = game();
                game.moving_left = false;
                game.moving_right = false;
                0
            }
            WM_SIZE => {
                let width = (lparam & 0xFFFF) as i32; // the new client size
                let height = ((lparam >> 16) & 0xFFFF) as i32;
                if width > 0 && height > 0 { // minimize sends 0x0; skip that
                    on_size(game(), window, width, height);
                }
                0
            }
            WM_GETMINMAXINFO => {
                // Let DefWindowProc fill the defaults first, then set Java's minimum size.
                DefWindowProcW(window, message, wparam, lparam);
                let info = lparam as *mut MinMaxInfo;
                (*info).min_track = [640, 480]; // window.setMinimumSize(640, 480)
                0
            }
            WM_ERASEBKGND => 1, // "already erased": the paint step fills everything
            WM_PAINT => {
                let game = game();
                let region_type = GetUpdateRgn(window, game.dirty_region, 0); // the dirty shape, used for clipping
                let mut ps: PaintStruct = core::mem::zeroed(); // Win32 fills this
                let screen = BeginPaint(window, &mut ps); // a DC clipped to the dirty rect
                if !game.back_dc.is_null() {
                    if region_type > NULLREGION {
                        SelectClipRgn(game.back_dc, game.dirty_region); // paint only inside the dirty shape
                    }
                    game.paint(game.back_dc, &ps.paint); // redraw the layers into the back buffer
                    let d = ps.paint;
                    BitBlt(screen, d.left, d.top, d.right - d.left, d.bottom - d.top,
                           game.back_dc, d.left, d.top, SRCCOPY); // flip just the dirty rect on screen
                    SelectClipRgn(game.back_dc, null_mut()); // release the clip
                }
                EndPaint(window, &ps);
                0
            }
            WM_DESTROY => {
                game().release_back_buffer(); // GDI handles we must not leak into exit
                PostQuitMessage(0); // end the message loop
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam), // everything else: the OS default
        }
    }
}

unsafe fn run() {
    unsafe {
        let instance = GetModuleHandleW(null()); // this exe's module handle
        let class_name = wz!("BreakoutRsWindow");
        let class = WndClass {
            style: 0, // no class-level styles
            wnd_proc: window_proc, // the pump defined above
            cls_extra: 0, // no extra bytes
            wnd_extra: 0,
            instance,
            icon: null_mut(), // default icon
            cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16), // the standard arrow
            background: GetStockObject(BLACK_BRUSH), // black base, same as Java's panel
            menu_name: null(), // no menu
            class_name,
        };
        if RegisterClassW(&class) == 0 {
            return; // can't register: don't try to open a window
        }

        let game = game();
        for (index, size) in FONT_SIZES.into_iter().enumerate() {
            // Negative height = em size, which is what Java's p sizes mean at 72 dpi.
            game.fonts[index] = CreateFontW(-size, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET,
                                            0, 0, CLEARTYPE_QUALITY, 0, wz!("Arial"));
        }
        game.dirty_region = CreateRectRgn(0, 0, 0, 0); // an empty region, refilled every WM_PAINT
        game.reset(); // lives/level/ball/messages, Java's resetGame

        // Java: frame.pack() gives an 800x600 client, setLocationRelativeTo(null) centres it.
        let mut bounds = Rect { left: 0, top: 0, right: 800, bottom: 600 };
        AdjustWindowRectEx(&mut bounds, WS_OVERLAPPEDWINDOW, 0, 0); // client 800x600 -> window bounds
        let (w, h) = (bounds.right - bounds.left, bounds.bottom - bounds.top); // the frame adds borders/title
        let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2; // centred on the work area
        let y = (GetSystemMetrics(SM_CYSCREEN) - h) / 2;
        let window = CreateWindowExW(0, class_name, wz!("Breakout (Rust)"), WS_OVERLAPPEDWINDOW,
                                     x, y, w, h, null_mut(), null_mut(), instance, null_mut());
        if window.is_null() {
            return; // no window: no game
        }
        game.window = window; // invalidations are sent through it
        ShowWindow(window, SW_SHOWNORMAL);
        UpdateWindow(window); // first paint before the loop starts

        // Sub-millisecond waits on Windows 10 1803+; plain ~15ms sleeps otherwise.
        let timer = CreateWaitableTimerExW(null(), null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS);
        let (mut freq, mut now) = (0i64, 0i64);
        QueryPerformanceFrequency(&mut freq); // the QPC rate, e.g. 10 MHz
        let frame = freq / FPS as i64; // one frame's worth of ticks
        QueryPerformanceCounter(&mut now);
        let mut next_frame = now; // the tick the next frame is due
        let mut msg: Msg = core::mem::zeroed(); // reused message pump buffer
        'running: loop {
            QueryPerformanceCounter(&mut now);
            if now < next_frame { // early this frame: sleep first
                let wait_100ns = (next_frame - now) * 10_000_000 / freq; // remaining ticks -> 100 ns units
                if !timer.is_null() {
                    let due = -wait_100ns; // negative = relative
                    SetWaitableTimer(timer, &due, 0, null(), null(), 0);
                    MsgWaitForMultipleObjects(1, &timer, 0, INFINITE, QS_ALLINPUT); // wake on timer OR input
                } else {
                    MsgWaitForMultipleObjects(0, null(), 0, (wait_100ns / 10_000).max(1) as u32, QS_ALLINPUT); // no hi-res timer: coarse ms wait
                }
            }

            // No TranslateMessage: nothing here reads WM_CHAR.
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'running; // the window closed
                }
                DispatchMessageW(&msg); // keys/mouse/paint handled by window_proc
            }

            QueryPerformanceCounter(&mut now);
            if now >= next_frame { // it's frame time (or we caught up late)
                if game.update() {
                    InvalidateRect(window, null(), 0); // a state change repaints the whole window
                }
                next_frame += frame;
                if next_frame <= now {
                    next_frame = now + frame; // fell behind: don't burst to catch up
                }
            }
        }
        // ExitProcess frees the fonts, region, timer and window.
    }
}

#[cfg(not(test))] // the tests keep the ordinary std entry points
#[unsafe(no_mangle)] // the linker finds the entry by this name
// The mingw-flavored linker derives the entry from the windows subsystem: WinMainCRTStartup.
extern "system" fn WinMainCRTStartup() -> ! {
    unsafe {
        run(); // the whole program is this loop
        ExitProcess(0) // normal end
    }
}

#[cfg(not(test))]
#[panic_handler] // no std panics: die like the JVM out-of-memory would
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { ExitProcess(1) }
}

// The linker wants this marker whenever floating point is used without a C runtime.
#[cfg(not(test))]
#[unsafe(no_mangle)]
static _fltused: i32 = 0;

#[cfg(test)]
mod tests {
    use super::*; // the game's own functions are the test surface

    #[test]
    fn series_math_matches_the_libm_values() { // sin/cos of our own series against libm constants
        assert!((sin(1.0) - 0.8414709848078965).abs() < 1e-12); // sin(1 rad)
        assert!((cos(1.0) - 0.5403023058681398).abs() < 1e-12); // cos(1 rad)
        assert!((sin(LAUNCH_TD) - 0.5735764363510460).abs() < 1e-12); // sin(35 deg)
        assert!((cos(LAUNCH_TD) - 0.8191520442889918).abs() < 1e-12); // cos(35 deg)
        assert!((sin(-BOUNCE_TD) + 0.8660254037844386).abs() < 1e-12); // sin(-60 deg)
        assert_eq!(sin(0.0), 0.0); // both series start at their flat term
        assert_eq!(cos(0.0), 1.0);
        assert_eq!((floor(-1.5), ceil(-1.5), floor(2.0), ceil(2.25)), (-2.0, -1.0, 2.0, 3.0)); // the hand-made rounding
    }

    #[test]
    fn launch_takes_the_level_speed() { // startLevel then Ball.launch
        let mut game = Game::new();
        game.width = 800; // the default panel
        game.paddle_y = 558; // height 600 - PADDLE_BOTTOM
        game.paddle_x = 400.0;
        game.start_level(3); // level 3
        assert_eq!(game.level, 3);
        assert_eq!(game.ball_speed, 5.6 + 3.0 * 0.3); // Game.startLevel's speed formula
        assert_eq!(game.ball.y, (558 - 16 - 1) as f64); // sitting on the paddle
        assert_eq!(game.ball.speed_x, 0.0); // and motionless
        assert_eq!(game.state, State::Ready);

        game.launch(1); // coin flip -> left
        let speed = (game.ball.speed_x * game.ball.speed_x + game.ball.speed_y * game.ball.speed_y).sqrt();
        assert!((speed - game.ball_speed).abs() < 1e-9); // the vector keeps Ball.speed
        assert!(game.ball.speed_y < 0.0); // up
        assert!(game.ball.speed_x / speed + sin(LAUNCH_TD) < 2e-12); // leftwards 35 degrees
    }

    #[test]
    fn paddle_bounce_angles_match_java() { // Ball.bounceOff's geometry
        let mut game = Game::new();
        game.ball_speed = 6.0;
        game.paddle_x = 400.0;
        game.paddle_y = 558;

        // Dead centre -> 0 degrees: straight up. The ball must overlap the paddle's box.
        game.ball = Ball { x: 400.0, y: 545.0, speed_x: -3.0, speed_y: 5.0 };
        game.bounce_off_paddle();
        assert!(game.ball.speed_x.abs() < 1e-12); // no sideways motion
        assert!((game.ball.speed_y + 6.0).abs() < 1e-12); // straight up at Ball.speed
        assert_eq!(game.ball.y, 541.0); // snapped 1 px above the paddle

        // The paddle's right edge -> 60 degrees: sin 60 across, cos 60 up.
        game.ball = Ball { x: 455.0, y: 550.0, speed_x: 2.0, speed_y: 5.0 };
        game.bounce_off_paddle();
        assert!((game.ball.speed_x - 0.8660254037844386 * 6.0).abs() < 1e-12);
        assert!((game.ball.speed_y + 0.5 * 6.0).abs() < 1e-12);

        // A rising ball is ignored (Ball.bounceOff returns when speed_y <= 0).
        game.ball = Ball { x: 400.0, y: 540.0, speed_x: 2.0, speed_y: -3.0 };
        game.bounce_off_paddle();
        assert_eq!(game.ball.speed_y, -3.0); // untouched
    }

    #[test]
    fn bricks_collide_like_java_and_score() { // BrickGrid.collide's resolution rules
        let mut game = Game::new();
        game.width = 800;
        game.paddle_y = 558;
        game.start_level(1);
        assert_eq!(game.brick_width, 65); // (800 - 100 - 45) / 10
        assert_eq!(game.brick_x(0), 50);
        assert_eq!(game.brick_y(1), 97); // 70 + 27
        assert_eq!(game.bricks_left, 50); // level 1 is a full wall

        // Left edge of the first brick: the smaller overlap is horizontal -> mirror x.
        game.ball = Ball { x: 38.0, y: 90.0, speed_x: 5.0, speed_y: -5.0 };
        assert_eq!(game.collide_bricks(), 50); // row 0 is worth (5 - 0) * 10
        assert_eq!(game.ball.x, 34.0); // pushed back to brick.x - radius
        assert_eq!(game.ball.speed_x, -5.0); // bouncing left
        assert_eq!(game.bricks_left, 49);
        assert!(!game.brick_alive[0]); // and the brick is gone

        // From below the wall, straight into one brick's body: the vertical overlap
        // wins; row 2 is 30 points.
        game.ball = Ball { x: 222.0, y: 150.0, speed_x: 1.0, speed_y: 5.0 };
        assert_eq!(game.collide_bricks(), 30);
        assert_eq!(game.ball.y, (146 + 16) as f64); // pushed under the brick
        assert_eq!(game.ball.speed_y, 5.0);

        // The last brick clears the wall (BrickGrid.isCleared).
        for cell in 0..BRICK_CELLS {
            game.brick_alive[cell] = false; // break every brick by hand
        }
        game.brick_alive[5] = true; // one brick left, top row (col 5 spans x 400..465)
        game.bricks_left = 1;
        game.ball = Ball { x: 437.5, y: 90.0, speed_x: 2.0, speed_y: 5.0 };
        assert_eq!(game.collide_bricks(), 50);
        assert_eq!(game.bricks_left, 0); // the wall is cleared
    }

    #[test]
    fn overlay_messages_match_game_java() { // drawMessage's branch texts
        let mut game = Game::new();
        game.score = 4321;
        game.level = 4;
        game.build_messages();
        assert_eq!(game.msg[0].buf.as_text(), "LEVEL 4");
        assert_eq!(game.msg[1].buf.as_text(), "BREAKOUT");
        assert_eq!(game.msg[2].buf.as_text(), "PRESS SPACE TO LAUNCH");

        game.state = State::LevelClear;
        game.build_messages();
        assert_eq!(game.msg[0].buf.as_text(), "LEVEL 4 CLEARED");
        assert_eq!(game.msg[1].buf.as_text(), "+400 BONUS"); // 100 * level

        game.state = State::GameOver;
        game.build_messages();
        assert_eq!(game.msg[0].buf.as_text(), "GAME OVER");
        assert_eq!(game.msg[1].buf.as_text(), "SCORE 4321");
        assert_eq!(game.msg[2].buf.as_text(), "PRESS SPACE TO PLAY AGAIN");

        game.state = State::Won;
        game.build_messages();
        assert_eq!(game.msg[0].buf.as_text(), "YOU WIN");

        game.state = State::Playing;
        game.build_messages();
        assert_eq!(game.msg[0].buf.as_text(), ""); // playing draws no overlay at all
    }

    #[test]
    fn level_layouts_match_brickgrid_java() { // the five layout tables
        let counts: [i32; 5] = [50, 25, 30, 30, 18];
        let mut game = Game::new();
        game.width = 800;
        game.paddle_y = 558;
        for level in 1..=5 {
            game.start_level(level);
            assert_eq!(game.bricks_left, counts[(level - 1) as usize], "level {level}");
            assert!((game.ball_speed - (5.6 + level as f64 * 0.3)).abs() < 1e-12); // speed climbs 0.3/level
        }
    }

    #[test]
    fn paddle_moves_and_clamps_like_java() { // Paddle.update's movement and clamps
        let mut game = Game::new();
        game.width = 800;
        game.paddle_x = 400.0;

        // Keys: exactly 9 px per tick.
        game.moving_right = true;
        game.paddle_update();
        assert_eq!(game.paddle_x, 409.0);

        // Mouse: at most 9 px per tick towards the target, then a snap.
        game.moving_right = false;
        game.mouse_target = 500.0;
        game.paddle_update();
        assert_eq!(game.paddle_x, 418.0);
        game.mouse_target = 421.0;
        game.paddle_update();
        assert_eq!(game.paddle_x, 421.0); // arrive exactly

        // The 8 px edge gap keeps the paddle fully on screen.
        game.mouse_target = -1.0;
        game.moving_right = true;
        for _ in 0..100 {
            game.paddle_update(); // push against the wall forever
        }
        assert_eq!(game.paddle_x, 800.0 - 8.0 - 55.0); // and sit at width - EDGE - half
    }
}
