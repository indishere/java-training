// Frogger DA Mogger â€” native Windows Rust rewrite of frogger.java (Processing).
//
// The Java sketch ran on a JVM with JOGL and repainted the whole window 60
// times a second. This file talks to Windows' own APIs instead: one window, a
// software renderer drawing into a DIB back buffer that only blits the parts
// of the screen that changed, a hand-written PNG decoder (inflate, unfilter,
// Java-parity resize) and no crates at all. There is no Rust standard library
// and no C runtime in the game binary: the exe imports only from DLLs that
// ship with Windows (raw-dylib, so no import libraries or Visual Studio are
// needed to link it). `cargo test` compiles this same file as a lib with std
// and links it normally so the unit tests can run through libtest.
#![cfg_attr(not(test), no_std)] // the game build has no standard library
#![cfg_attr(not(test), no_main)] // and no CRT startup: we hand-write the entry
#![windows_subsystem = "windows"] // GUI exe: no console window opens with it
#![cfg_attr(test, allow(dead_code))] // tests don't call every game helper
#![allow(unsafe_op_in_unsafe_fn)] // unsafe fns' bodies already promise Win32 discipline line by line

use core::cell::UnsafeCell; // shared mutable state cell (single-threaded game)
use core::ffi::c_void; // Windows handles and opaque pointers are **mut c_void
use core::ptr::{null, null_mut}; // null handled ptr helpers for Win32 calls

// Window: the Java sketch's fixed 1900x1000 window, 60 FPS target.
const FPS: i32 = 60; // Java: framesPerSecond â€” frames the game targets
const WINDOW_W: i32 = 1900; // Java: windowWidth â€” client area width
const WINDOW_H: i32 = 1000; // Java: windowHeight â€” client area height

// Player (Steve). His picture is 160x291; it is resized to 135 tall with the
// Java resizeSharp integer maths, which gives 160 * 135 / 291 = 74 wide.
const PLAYER_SRC_W: i32 = 160; // steve.png's own width in pixels
const PLAYER_SRC_H: i32 = 291; // steve.png's own height in pixels
const PLAYER_H: i32 = 135; // Java: playerHeight â€” Steve's on-screen height
const PLAYER_W: i32 = 74; // resized width: 160 * 135 / 291 = 74 (integer)
const PLAYER_SPEED: i32 = 3; // Java: playerSpeed â€” pixels per step
// The starter unit: Java's playerSize = 75, which the gaps are built on.
const PLAYER_UNIT: i32 = 75; // Java: playerSize â€” the sizing unit
const PLAYER_START_GAP: i32 = PLAYER_UNIT / 2; // Java: playerStartGap = 37
const LOG_GAP: i32 = PLAYER_UNIT * 2; // Java: logGap = 150, closest two logs
const VEHICLE_GAP: i32 = PLAYER_UNIT * 2; // Java: vehicleGap = 150, closest two vehicles

// Jump (the water and anything lower than the current jump height can't hit Steve).
const JUMP_LENGTH: i32 = 240; // Java: jumpLength â€” pixels flown in a full jump
const JUMP_HEIGHT: i32 = 60; // Java: jumpHeight â€” height at the top of the arc
const JUMP_FRAMES: i32 = 250 * FPS / 1000; // Java: jumpFrames â€” 250 ms in frames = 15
const JUMP_COOLDOWN: i64 = 1000; // Java: jumpCooldown â€” ms between jumps
const JUMP_GROWTH: i32 = PLAYER_H / 5; // Java: jumpGrowth â€” how much bigger Steve looks (27)
const SQRT_2: f64 = 1.4142135623730951; // diagonal steps shrink by 1 / sqrt(2)

// Lanes. placeLanes(): Road One, gap, River, gap, Road Two centred vertically.
const ROAD_HEIGHT: i32 = 150; // Java: roadHeight â€” each road band's height
const LANE_GAP: i32 = PLAYER_H; // Java: laneGap = 135, breathing room around the river
const RIVER_HEIGHT: i32 = ROAD_HEIGHT; // Java: riverHeight â€” the river band's height
const ROAD_ONE_Y: i32 = (WINDOW_H - (ROAD_HEIGHT * 2 + LANE_GAP * 2 + RIVER_HEIGHT)) / 2; // Java: roadOneY = 140
const RIVER_Y: i32 = ROAD_ONE_Y + ROAD_HEIGHT + LANE_GAP; // Java: riverY = 425
const ROAD_TWO_Y: i32 = RIVER_Y + RIVER_HEIGHT + LANE_GAP; // Java: roadTwoY = 710

// Sprites. Every vehicle picture is 2172x724 and loads once, resized to 140.
const VEHICLE_SRC_W: i32 = 2172; // the shared source width of all vehicle PNGs
const VEHICLE_SRC_H: i32 = 724; // the shared source height of all vehicle PNGs
const VEHICLE_PADDING: i32 = 5; // Java: vehiclePadding â€” road edge padding
const VEHICLE_H: i32 = ROAD_HEIGHT - VEHICLE_PADDING * 2; // Java: vehicleHeight = 140
const VEHICLE_W: i32 = VEHICLE_SRC_W * VEHICLE_H / VEHICLE_SRC_H; // resized width: exactly 420
const MAX_VEHICLES: usize = 10; // Java: maxVehicles â€” the vehicles[] array size

// Logs. Picture sizes: birch 1167x445, mossy 1200x503, oak 1213x511, short 835x391.
// Every spawned log is resized to a random height, so the four originals stay
// decoded and each log slot owns its own resized copy.
const MAX_LOG_H: i32 = (RIVER_HEIGHT * 3) / 4; // Java: maxLogHeight = 112
const MIN_LOG_H: i32 = MAX_LOG_H * 3 / 4; // Java: minLogHeight = 84
const MIN_LOG_SPEED: i32 = 2; // Java: minLogSpeed â€” slowest log, px per frame
const MAX_LOG_SPEED: i32 = 4; // Java: maxLogSpeed â€” fastest log, px per frame
const MAX_LOGS: usize = 10; // Java: maxLogs â€” the logs[] array size
// Widest a log can ever get: birch at 112 tall is 1167 * 112 / 445 = 293 wide.
const LOG_SLOT_W: i32 = 320; // slot buffer width, rounded up to fit every type
const LOG_SLOT_H: i32 = MAX_LOG_H; // slot buffer height, the tallest log

// Spawning delays (Java: in milliseconds before the next vehicle/log enters).
const MIN_SPAWN_DELAY: i32 = 2000; // Java: minSpawnDelay
const MAX_SPAWN_DELAY: i32 = 4500; // Java: maxSpawnDelay (exclusive upper bound)
const ROAD_TWO_SPAWN_OFFSET: i64 = 1000; // Java: roadTwoSpawnOffset
const LOG_SPAWN_DELAY: i64 = 3000; // Java: logSpawnDelay

// Traffic: speeds per vehicle type are picked from these Java tables.
const MIN_VEHICLE_SPEEDS: [i32; 3] = [5, 3, 3]; // Java: minVehicleSpeeds (car, truck, bus)
const MAX_VEHICLE_SPEEDS: [i32; 3] = [10, 7, 6]; // Java: maxVehicleSpeeds (car, truck, bus)
const VEHICLE_TALLNESS: [i32; 3] = [40, 100, 90]; // Java: vehicleTallness â€” jump height needed

// Collision: a pixel only counts when its alpha passes this Java threshold.
const MIN_VISIBLE_ALPHA: u32 = 128; // Java: minVisibleAlpha

// Debug mode: Shift+D toggles it while playing, exactly like the Java game.
const DEBUG_MODE: bool = true; // Java: DEBUG_MODE â€” gated feature flag
const HITBOX_COLOR: u32 = 0x78FF0000; // Java: hitboxColor â€” see-through red
const OUTLINE_COLOR: u32 = 0xFFFFFF00; // Java: boxOutlineColor â€” solid yellow

// FPS counter text, drawn top-left: "FPS: n/60" at size 20, 10 px in.
const FPS_SIZE: i32 = 20; // Java: fpsTextSize â€” Arial Bold em size
const FPS_MARGIN: i32 = 10; // Java: fpsMargin â€” distance from the top-left corner
const FPS_TEXT_W: i32 = 150; // room for the widest "FPS: 100/60" plus padding
const FPS_TEXT_H: i32 = 26; // one text line's height, generous on purpose
const FPS_RECT: Rect = Rect { x0: FPS_MARGIN, y0: FPS_MARGIN, x1: FPS_MARGIN + FPS_TEXT_W, y1: FPS_MARGIN + FPS_TEXT_H }; // the redraw box for the counter

// Colours, Java style: 0xAARRGGBB. DIB pixels are BGRA bytes in memory, so the
// same u32 value reads back as 0xAARRGGBB once stored as a u32 on the buffer.
const BG_COLOR: u32 = 0xFF000000; // Java: backgroundColor â€” black background
const ROAD_COLOR: u32 = 0xFF404040; // Java: roadColor â€” dark gray
const RIVER_COLOR: u32 = 0xFF1E6BD6; // Java: riverColor â€” blue water

type H = *mut c_void; // every Windows handle type is just a pointer

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)] // tests assert rectangles with assert_eq!
struct Rect { // rectangle with an exclusive right/bottom, like our own math
    x0: i32, // left edge (inclusive)
    y0: i32, // top edge (inclusive)
    x1: i32, // right edge (exclusive)
    y1: i32, // bottom edge (exclusive)
}

impl Rect {
    const EMPTY: Rect = Rect { x0: 0, y0: 0, x1: 0, y1: 0 }; // nothing drawn here
    const FULL: Rect = Rect { x0: 0, y0: 0, x1: WINDOW_W, y1: WINDOW_H }; // the whole client area

    fn is_empty(self) -> bool { // true when the rectangle covers no pixels at all
        self.x0 >= self.x1 || self.y0 >= self.y1 // zero or negative width/height
    }

    fn union_with(self, o: Rect) -> Rect { // smallest rectangle covering both
        Rect {
            x0: self.x0.min(o.x0), // leftmost of the two lefts
            y0: self.y0.min(o.y0), // topmost of the two tops
            x1: self.x1.max(o.x1), // rightmost of the two rights
            y1: self.y1.max(o.y1), // bottommost of the two bottoms
        }
    }

    fn clamp_client(self) -> Rect { // cut the rectangle down to the window's client area
        Rect {
            x0: self.x0.max(0), // nothing left of the screen
            y0: self.y0.max(0), // nothing above the screen
            x1: self.x1.min(WINDOW_W), // nothing right of the screen
            y1: self.y1.min(WINDOW_H), // nothing below the screen
        }
    }

    // Java's Vehicle/Log boxes: [x, x + w) x [y, y + h) with long arithmetic in i32.
    fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x0: x, y0: y, x1: x + w, y1: y + h } // build from left/top/width/height
    }
}

// ---- Win32: only the calls this game makes, declared by hand so no crates are needed. ----
// Nearly every import below is tagged with a raw-dylib link: rustc writes the
// import table itself straight from the DLL name, no .lib files anywhere.

#[repr(C)]
struct WndClass { // WNDCLASSW for RegisterClassW
    style: u32, // class-wide style bits (we don't need any)
    wnd_proc: unsafe extern "system" fn(H, u32, usize, isize) -> isize, // our event callback
    cls_extra: i32, // extra class bytes: 0
    wnd_extra: i32, // extra window bytes: 0
    instance: H, // the owning module (this exe)
    icon: H, // icon handle: null = default
    cursor: H, // cursor handle: the stock arrow
    background: H, // background brush: null here, we fill pixels ourselves
    menu_name: *const u16, // no menu
    class_name: *const u16, // this class's UTF-16 name pointer
}

#[repr(C)]
struct Msg { // MSG for PeekMessageW/DispatchMessageW
    window: H, // the window the message belongs to
    message: u32, // the message code (WM_PAINT, WM_KEYDOWN, ...)
    wparam: usize, // first parameter (e.g. the virtual key)
    lparam: isize, // second parameter (e.g. key repeat bits)
    time: u32, // posting time
    pt: [i32; 2], // cursor position at posting time
    private: u32, // Windows uses this internally; zeroed by us
}

#[repr(C)]
struct PaintStruct { // PAINTSTRUCT for BeginPaint/EndPaint
    hdc: H, // the device context to paint into
    erase: i32, // 1 when Windows wants a full background erase
    paint: Rect, // the dirty area (same i32 layout as Win32's RECT)
    restore: i32, // private
    inc_update: i32, // private
    reserved: [u8; 32], // private padding
}

#[repr(C)]
struct BmiHeader { // BITMAPINFOHEADER, the header of a 32bpp DIB request
    size: u32, // size of this header in bytes (40)
    width: i32, // bitmap width in pixels
    height: i32, // negative: top-down rows (row 0 is the top row)
    planes: u16, // must be 1
    bit_count: u16, // bits per pixel: 32
    compression: u32, // BI_RGB: raw BGRA pixels, no compression
    size_image: u32, // 0 lets Windows compute it
    x_pels: i32, // dots per inch, unused
    y_pels: i32, // dots per inch, unused
    clr_used: u32, // palette entries: 0 for true colour
    clr_important: u32, // palette entries: 0
}

#[repr(C)]
struct Bmi { // BITMAPINFO: header plus colour table (empty here)
    header: BmiHeader, // the layout above
    colors: [u32; 1], // colour table stub, unused for 32bpp
}

const WM_DESTROY: u32 = 0x0002; // window shutting down
const WM_ERASEBKGND: u32 = 0x0014; // Windows asks to erase the background
const WM_PAINT: u32 = 0x000F; // part of the window needs drawing
const WM_KEYDOWN: u32 = 0x0100; // a key was pressed
const WM_KEYUP: u32 = 0x0101; // a key was released
const WM_KILLFOCUS: u32 = 0x0008; // window lost keyboard focus
const PM_REMOVE: u32 = 1; // PeekMessageW: consume the message
const QS_ALLINPUT: u32 = 0x04FF; // MsgWait: wake on any input message
const INFINITE: u32 = 0xFFFF_FFFF; // MsgWait: wait forever for the timer
const WS_OVERLAPPED: u32 = 0x0000_0000; // plain single window
const WS_CAPTION: u32 = 0x00C0_0000; // title bar + border
const WS_SYSMENU: u32 = 0x0008_0000; // close button on the title bar
const WS_MINIMIZEBOX: u32 = 0x0002_0000; // allow minimise; no resize styles
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32; // let Windows pick the position
const SW_SHOW: i32 = 5; // ShowWindow: show and activate
const NULL_BRUSH: i32 = 5; // GetStockObject id for the empty brush (no repaint fill by GDI)
const IDC_ARROW: usize = 32512; // LoadCursorW id for the standard arrow cursor
const DIB_RGB_COLORS: u32 = 0; // CreateDIBSection: palette is colours, not indexes
const BI_RGB: u32 = 0; // uncompressed BGRA pixel format
const SRCCOPY: u32 = 0x00CC_0020; // BitBlt: plain pixel copy
const GENERIC_READ: u32 = 0x8000_0000; // CreateFileW: read access
const FILE_SHARE_READ: u32 = 1; // CreateFileW: others may read too
const OPEN_EXISTING: u32 = 3; // CreateFileW: file must already exist
const INVALID_HANDLE: H = usize::MAX as H; // CreateFileW's failure value (-1)
const MEM_COMMIT: u32 = 0x1000; // VirtualAlloc: commit pages
const MEM_RESERVE: u32 = 0x2000; // VirtualAlloc: reserve address space
const MEM_RELEASE: u32 = 0x8000; // VirtualFree: release the whole region
const PAGE_READWRITE: u32 = 4; // VirtualAlloc: read-write pages
const CREATE_WAITABLE_TIMER_HIGH_RESOLUTION: u32 = 0x0000_0002; // sub-ms precision timer on Win10 1803+
const TIMER_ALL_ACCESS: u32 = 0x001F_0003; // full rights on the timer handle
const TRANSPARENT: i32 = 1; // SetBkMode: don't paint a box behind text
const TA_LEFT: u32 = 0; // TextOutW x is the left edge
const TA_TOP: u32 = 0; // TextOutW y is the top edge
const FW_BOLD: i32 = 700; // CreateFontW weight for bold
const DEFAULT_CHARSET: u32 = 1; // CreateFontW charset
const CLEARTYPE_QUALITY: u32 = 5; // CreateFontW: ClearType rendering

// Virtual-key codes used by the game (E/S/D/F follow the Java game's WASD-ish
// layout; the arrows match Processing's UP/DOWN/LEFT/RIGHT constants).
const VK_SHIFT: usize = 0x10; // shift key (gate for the debug toggle)
const VK_CAPITAL: usize = 0x14; // caps lock (also gates the debug toggle)
const VK_SPACE: usize = 0x20; // space bar = jump
const VK_LEFT: usize = 0x25; // left arrow = walk left
const VK_UP: usize = 0x26; // up arrow = walk up
const VK_RIGHT: usize = 0x27; // right arrow = walk right
const VK_DOWN: usize = 0x28; // down arrow = walk down

#[cfg_attr(all(windows, not(test)), link(name = "user32", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "user32"))] // test builds use the normal import library
unsafe extern "system" { // user32 calls used by the game
    fn RegisterClassW(class: *const WndClass) -> u16; // register our window class
    fn CreateWindowExW(ex_style: u32, class: *const u16, title: *const u16, style: u32,
                       x: i32, y: i32, w: i32, h: i32,
                       parent: H, menu: H, instance: H, param: H) -> H; // create the game window
    fn DefWindowProcW(window: H, msg: u32, wparam: usize, lparam: isize) -> isize; // default handling for ignored messages
    fn ShowWindow(window: H, cmd: i32) -> i32; // make the window visible
    fn UpdateWindow(window: H) -> i32; // force the first paint now
    fn PeekMessageW(msg: *mut Msg, window: H, min: u32, max: u32, remove: u32) -> i32; // pump one pending message
    fn DispatchMessageW(msg: *const Msg) -> isize; // hand a message to window_proc
    fn PostQuitMessage(code: i32); // queue WM_QUIT to leave the loop
    fn MsgWaitForMultipleObjects(count: u32, handles: *const H, wait_all: i32, ms: u32, wake: u32) -> u32; // sleep until timer or message
    fn GetDC(window: H) -> H; // borrow the window's device context
    fn ReleaseDC(window: H, dc: H) -> i32; // give the device context back
    fn BeginPaint(window: H, ps: *mut PaintStruct) -> H; // start painting: also validates the area
    fn EndPaint(window: H, ps: *const PaintStruct) -> i32; // finish painting
    fn LoadCursorW(instance: H, name: *const u16) -> H; // load the arrow cursor
    fn GetKeyState(key: i32) -> i16; // sticky key state (shift/caps for the debug toggle)
    fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex_style: u32) -> i32; // client rect -> window rect
}

#[cfg_attr(all(windows, not(test)), link(name = "gdi32", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "gdi32"))] // test builds use the normal import library
unsafe extern "system" { // gdi32 calls used by the game
    fn GetStockObject(index: i32) -> H; // shared stock objects (brushes)
    fn SelectObject(dc: H, object: H) -> H; // pick a brush/font/bitmap into a dc
    fn CreateCompatibleDC(dc: H) -> H; // memory dc for the back buffer
    fn CreateDIBSection(dc: H, bmi: *const Bmi, usage: u32, bits: *mut *mut u32,
                        section: H, offset: u32) -> H; // allocate the 32bpp pixel buffer
    fn BitBlt(dst: H, x: i32, y: i32, w: i32, h: i32, src: H, sx: i32, sy: i32, rop: u32) -> i32; // copy pixels (the blit)
    fn CreateFontW(height: i32, width: i32, escapement: i32, orientation: i32, weight: i32,
                   italic: u32, underline: u32, strike_out: u32, charset: u32,
                   out_precision: u32, clip_precision: u32, quality: u32,
                   pitch_and_family: u32, face: *const u16) -> H; // the FPS counter font
    fn TextOutW(dc: H, x: i32, y: i32, text: *const u16, len: i32) -> i32; // draw the FPS text
    fn SetTextColor(dc: H, color: u32) -> u32; // glyph colour (COLORREF)
    fn SetBkMode(dc: H, mode: i32) -> i32; // no opaque box behind the glyphs
    fn SetTextAlign(dc: H, align: u32) -> u32; // x,y = top-left of the first glyph
}

#[cfg_attr(all(windows, not(test)), link(name = "kernel32", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "kernel32"))] // test builds use the normal import library
unsafe extern "system" { // kernel32 calls used by the game
    fn QueryPerformanceCounter(count: *mut i64) -> i32; // high-resolution clock tick
    fn QueryPerformanceFrequency(freq: *mut i64) -> i32; // clock ticks per second
    fn GetTickCount64() -> u64; // Java's millis(): ms since boot
    fn CreateWaitableTimerExW(attributes: *const c_void, name: *const u16, flags: u32, access: u32) -> H; // frame timer
    fn SetWaitableTimer(timer: H, due: *const i64, period: i32,
                        completion: *const c_void, arg: *const c_void, resume: i32) -> i32; // arm the timer once
    fn CreateFileW(name: *const u16, access: u32, share: u32, security: *const c_void,
                   disposition: u32, flags: u32, template: H) -> H; // open a PNG file
    fn ReadFile(file: H, buffer: *mut u8, len: u32, read: *mut u32, overlapped: *mut c_void) -> i32; // read its bytes
    fn CloseHandle(handle: H) -> i32; // close a file/timer handle
    fn GetModuleHandleW(name: *const u16) -> H; // this exe's module handle (null)
    fn GetModuleFileNameW(module: H, name: *mut u16, size: u32) -> u32; // full path of the exe
    fn VirtualAlloc(address: *mut c_void, size: usize, alloc: u32, protect: u32) -> *mut c_void; // decode scratch memory
    fn VirtualFree(address: *mut c_void, size: usize, free: u32) -> i32; // release it after load
    fn ExitProcess(code: u32) -> !; // quit the process immediately
}

// Some large sprite copies compile to CRT memcpy/memmove/memset/memcmp calls.
// ntdll is mapped into every Windows process anyway, so import exactly those
// from it via raw-dylib too (nothing else and nothing unused gets imported).
#[cfg(all(windows, not(test)))]
#[link(name = "ntdll", kind = "raw-dylib")]
#[allow(dead_code)] // only the ones the codegen happens to emit are used
unsafe extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void; // non-overlapping copy
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void; // overlap-safe copy
    fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void; // fill n bytes
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32; // byte compare
}

// ASCII string literal -> null-terminated UTF-16 array at compile time, plus a
// *const u16 pointer back into it. Windows string arguments want exactly this.
macro_rules! wz {
    ($s:literal) => {{
        const S: &str = concat!($s, "\0"); // the literal plus a terminating NUL
        const W: [u16; S.len()] = { // widened at compile time, no runtime cost
            let (b, mut out, mut i) = (S.as_bytes(), [0u16; S.len()], 0); // scan the ASCII bytes
            while i < b.len() { // each byte fits u16 unchanged
                out[i] = b[i] as u16; // widen one byte
                i += 1; // move on
            }
            out // the widened copy (the const's value; naming it W would be a self-cycle)
        };
        &W as *const [u16; S.len()] as *const u16 // pointer to the permanent array
    }};
}

// Shared mutable state. The game, its images and its buffers are only ever
// touched from the one thread that owns the window, so &mut handouts are safe.
struct Global<T>(UnsafeCell<T>); // UnsafeCell wrapper with a Sync marker
unsafe impl<T> Sync for Global<T> {} // safe: single-threaded game by construction
impl<T> Global<T> {
    fn get(&self) -> &mut T { // hand out a &mut from a &self (caller enforces the rules)
        unsafe { &mut *self.0.get() } // reborrow the raw cell pointer
    }
}


// Java's Processing round() is floor(x + 0.5) for this game's non-negative
// values (the jump's diagonal shrink), so keep the exact same integer result.
fn round_half_up(x: f64) -> i32 {
    (x + 0.5) as i32 as i32 // shift up by a half then truncate
}

// xorshift64*, seeded from the performance counter at startup: enough spread
// for the type/speed/height picks, and no heap allocator anywhere.
static RNG: Global<u64> = Global(UnsafeCell::new(1)); // never zero: xorshift64* requires it

fn rng_next() -> u64 { // one pseudorandom 64-bit draw
    let rng = RNG.get(); // the shared state
    let mut x = *rng; // current value
    x ^= x >> 12; // xorshift step 1
    x ^= x << 25; // xorshift step 2
    x ^= x >> 27; // xorshift step 3
    *rng = x; // store back for the next call
    x.wrapping_mul(0x2545_F491_4F6C_DD1D) // the * multiplier of xorshift64*
}

// Java's (int) random(min, maxExclusive): a uniform integer in [min, maxExclusive).
fn rng_i32(min: i32, max_exclusive: i32) -> i32 {
    let span = (max_exclusive - min).max(1) as u64; // how many values to spread over
    min + ((rng_next() >> 32) * span >> 32) as i32 // high bits multiplied down into the span
}

// Java's millis(): relative time in milliseconds since the machine booted.
fn millis() -> i64 {
    unsafe { GetTickCount64() as i64 } // u64 -> i64, plenty of uptime headroom
}

// ---- PNG loading: decode, resize and flatten every sprite once at startup ----
//
// Java loads 11 PNG files (each one 8-bit, colour type 6 RGBA, not interlaced)
// and runs its own nearest-neighbour resizeSharp over them. This does the same
// without crates: walk the file's chunks, hand-inflate the zlib stream into a
// row-strided buffer, PNG-unfilter it in place, then copy pixels straight out
// of that buffer into the final sprites. The scratch space is one VirtualAlloc
// that is released after loading, so the running game allocates nothing.

const FILE_CAP: usize = 1_600_000; // lookahead: no PNG here is bigger than ~1.06 MB
const ZSTRIDE_CAP: usize = (VEHICLE_SRC_W as usize * 4 + 1) * VEHICLE_SRC_H as usize; // 6,290,836 bytes: the biggest inflate

struct Scratch { // one VirtualAlloc split into a file area and an inflate area
    file: *mut u8, // PNG bytes land here (read + IDAT packing share it)
    zs: *mut u8, // decoded (inflated, unfiltered) pixels land here
}
static SCRATCH: Global<Scratch> = Global(UnsafeCell::new(Scratch { file: null_mut(), zs: null_mut() })); // starts empty

unsafe fn ensure_scratch() { // allocate the decode scratch once (also used by the tests)
    let s = SCRATCH.get(); // the shared record
    if s.file.is_null() { // only allocate the first time
        let mem = VirtualAlloc(null_mut(), FILE_CAP + ZSTRIDE_CAP, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE); // one region, both halves
        s.file = mem as *mut u8; // first half: the file bytes
        s.zs = (mem as *mut u8).add(FILE_CAP); // second half: the inflated rows
    }
}

// -- bit reader + DEFLATE (stored / fixed / dynamic Huffman), puff-style ----

struct BitReader<'a> { // reads the deflate bit stream, LSB first like RFC 1951
    data: &'a [u8], // the raw zlib payload bytes
    pos: usize, // next byte to feed the bit buffer
    bits: u64, // pending bits (LSB = oldest bit)
    cnt: u32, // how many pending bits that buffer still holds
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self { // start a reader at the first byte
        BitReader { data, pos: 0, bits: 0, cnt: 0 } // nothing buffered yet
    }

    fn byte_at(&self, at: usize) -> Result<u8, ()> { // fetch one raw byte or fail past the end
        self.data.get(at).copied().ok_or(()) // None means the stream ended too early
    }

    fn bit(&mut self) -> Result<u32, ()> { // pull a single bit (low bit first)
        if self.cnt == 0 { // buffer empty: load the next byte
            self.bits = self.byte_at(self.pos)? as u64; // whole byte becomes pending bits
            self.pos += 1; // advance the stream
            self.cnt = 8; // eight bits now available
        }
        let b = (self.bits & 1) as u32; // the oldest bit is the result
        self.bits >>= 1; // drop it from the buffer
        self.cnt -= 1; // and mark it consumed
        Ok(b) // decoded bit, 0 or 1
    }

    fn bits(&mut self, n: u32) -> Result<u32, ()> { // pull n bits, first bit = lowest
        let mut v = 0u32; // the little-endian assembly
        for i in 0..n { // RFC 1951: extra bits are stored LSB-first
            v |= self.bit()? << i; // shift each bit into place
        }
        Ok(v) // the assembled small integer
    }

    fn align(&mut self) { // drop the pending bits to reach the next byte boundary
        self.cnt = 0; // stored uses whole bytes, so unaligned partial bits are wasted
    }

    fn stored(&mut self) -> Result<u32, ()> { // read a stored block's LEN header
        self.align(); // blocks start on byte boundaries for stored
        let len = self.byte_at(self.pos)? as u32 | (self.byte_at(self.pos + 1)? as u32) << 8; // LEN is little-endian
        let nlen = self.byte_at(self.pos + 2)? as u32 | (self.byte_at(self.pos + 3)? as u32) << 8; // NLEN = the one's complement
        if len ^ 0xFFFF != nlen { // NLEN must equal ~LEN
            return Err(()); // corrupt header: the stream is broken
        }
        self.pos += 4; // skip past the two header words
        Ok(len) // how many raw bytes follow
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ()> { // copy n whole stored bytes
        let end = self.pos + n; // where to stop
        let at = self.pos; // where they start
        if end > self.data.len() { // past the end of the stream
            return Err(()); // corrupt: refuse to overrun
        }
        self.pos = end; // the caller received those bytes
        Ok(&self.data[at..end]) // slice pointing straight into the stream
    }
}

const MAXBITS: usize = 15; // deflate's longest code is 15 bits

struct Huff { // puff.c's tiny Huffman table: counts per length + sorted symbols
    counts: [u16; MAXBITS + 1], // how many codes have each length 0..15
    symbols: [u16; 320], // symbol numbers ordered by (length, symbol)
}
impl Huff {
    const fn empty() -> Self { // an all-zero table (decode always fails, legal)
        Huff { counts: [0; 16], symbols: [0; 320] } // compiler-known zeros
    }
}

// Build a Huffman table from code lengths (puff's approach: counts plus a
// symbol list sorted by (length, symbol); incomplete tables stay legal â€” the
// decode below simply fails on any code that was never assigned).
fn build(h: &mut Huff, lengths: &[u8]) {
    h.counts = [0; 16]; // zero all the length counters first
    let total = lengths.len() as u16; // how many symbols the alphabet has
    for &l in lengths { // count one more code per symbol
        h.counts[l as usize] += 1; // index = the code length itself
    }
    h.symbols = [0; 320]; // reset the ordering list
    if h.counts[0] == total { // every symbol got length 0
        return; // empty table: every decode fails, and that is legal
    }
    let mut left: i32 = 1; // leftover slots at the previous code length
    for len in 1..=MAXBITS { // walk all 15 lengths for the oversubscription check
        left <<= 1; // a full set doubles the available slots
        left -= h.counts[len] as i32; // this length consumes its share
        if left < 0 { // oversubscribed: more codes than bit patterns
            return; // degenerate table, decode will fail everywhere it hits
        }
    }
    let mut offs = [0u16; 16]; // start index of each length inside symbols[] (offs[0] and offs[1] stay zero)
    for len in 1..MAXBITS { // fill offs[2..=15]: puff's rule, symbols of a length come after the previous length's
        offs[len + 1] = offs[len] + h.counts[len]; // (never count[len-1]: the zero-length count would shift everything)
    }
    for (sym, &l) in lengths.iter().enumerate() { // second pass: place every symbol
        if l != 0 { // length-0 symbols have no code at all
            h.symbols[offs[l as usize] as usize] = sym as u16; // ordered: first by length, then by value
            offs[l as usize] += 1; // next symbol of the same length goes after
        }
    }
}

fn decode(h: &Huff, br: &mut BitReader) -> Result<u32, ()> { // deflate's canonical bit decoder
    let mut code: i32 = 0; // the running code value (puff style)
    let mut first: i32 = 0; // first code value of the current length
    let mut index: usize = 0; // index into the sorted symbols
    for len in 1..=MAXBITS { // try lengths 1..15
        code |= br.bit()? as i32; // one more bit arrives, shifted later
        let count = h.counts[len] as i32; // how many codes exist at this length
        if code < first + count { // does the code fall inside this length's range?
            return Ok(h.symbols[index + (code - first) as usize] as u32); // yes: that's the symbol
        }
        index += count as usize; // skip this length's symbols
        first = (first + count) << 1; // next length's first code, binary-tree style
        code <<= 1; // push the running code left for the next bit
    }
    Err(()) // a 16-bit string is never legal: corrupt stream
}

static FIXED_READY: Global<bool> = Global(UnsafeCell::new(false)); // build once flag
static FIXED: Global<[Huff; 2]> = Global(UnsafeCell::new([Huff::empty(), Huff::empty()])); // [literal/length, distances]

fn ensure_fixed() { // build RFC 1951's fixed code tables on first use
    if *FIXED_READY.get() { // already built in this process
        return;
    }
    let mut lit_len = [0u8; 288]; // code lengths for symbols 0..287
    for (l, entry) in lit_len.iter_mut().enumerate() { // each symbol's fixed length
        *entry = match l { // the fixed table's rule
            0..=143 => 8, // most literals: 8 bits
            144..=255 => 9, // high literals: 9 bits
            256..=279 => 7, // low symbols incl. end-of-block: 7 bits
            _ => 8, // the rest of the high range: 8 bits
        };
    }
    let fixed = FIXED.get(); // the shared pair of tables
    build(&mut fixed[0], &lit_len); // literal/length codes
    build(&mut fixed[1], &[5u8; 32]); // 32 five-bit distance codes (only 0..29 legal)
    *FIXED_READY.get() = true; // never rebuild again
}

// RFC 1951 tables for LZ77 lengths (extra bits included) and distances.
static LENS: [u32; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258]; // base match lengths for codes 257..285
static LEXT: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0]; // their extra bit widths
static DSTS: [u32; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577]; // base match distances for codes 0..29
static DEXT: [u32; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13]; // their extra bit widths
static LORDER: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]; // the scramble order code lengths are written in

// Decode the dynamic table definitions (HLIT/HDIST/HCLEN plus the code length
// codes) and return the literal/length and distance code length lists.
fn read_dynamic(br: &mut BitReader) -> Result<([u8; 288], [u8; 30]), ()> {
    let hlit = br.bits(5)? as usize + 257; // 5 bits: literals + length codes = 257..286
    let hdist = br.bits(5)? as usize + 1; // 5 bits: distance codes = 1..30
    let hclen = br.bits(4)? as usize + 4; // 4 bits: code length codes = 4..19
    if hlit > 286 || hdist > 30 { // sizes beyond the tables are illegal
        return Err(()); // corrupt block header
    }
    let mut clen = [0u8; 19]; // lengths for the code-length alphabet
    for i in 0..hclen { // the first HCLEN entries (3 bits each)
        clen[LORDER[i] as usize] = br.bits(3)? as u8; // each writes into its scrambled slot
    }
    let mut hcl = Huff::empty(); // the alphabet that spells the next code lengths
    build(&mut hcl, &clen); // build that bootstrap table
    let mut lens = [0u8; 288 + 30]; // both alphabets' lengths, written back to back
    let mut n = 0usize; // how many entries written so far
    while n < hlit + hdist { // until both tables are filled
        let sym = decode(&hcl, br)?; // next code-length symbol drives this match
        match sym { // RFC 1951's four metacodes
            0..=15 => { // a literal code length for the next table entry
                if n >= lens.len() { // running past the end is corrupt
                    return Err(()); // refuse to overwrite
                }
                lens[n] = sym as u8; // the symbol value IS the code length
                n += 1; // one entry written
            }
            16 => { // repeat the previous length 3..6 more times
                if n == 0 { // repeats must follow a first entry
                    return Err(()); // corrupt stream
                }
                let rep = 3 + br.bits(2)? as usize; // 2 extra bits, biased by 3
                if n + rep > lens.len() { // the run must fit the tables
                    return Err(()); // corrupt
                }
                let prev = lens[n - 1]; // the value being stamped again
                for _ in 0..rep { // puff copies the prior entry verbatim
                    lens[n] = prev; // stamp it
                    n += 1; // one entry at a time
                }
            }
            17 => { // skip 3..10 zeros, packed tight
                n += 3 + br.bits(3)? as usize; // 3 extra bits, biased by 3
                if n > lens.len() { // past the end of the tables
                    return Err(()); // corrupt
                }
            }
            18 => { // skip 11..138 zeros, packed tighter
                n += 11 + br.bits(7)? as usize; // 7 extra bits, biased by 11
                if n > lens.len() { // past the end of the tables
                    return Err(()); // corrupt
                }
            }
            _ => return Err(()), // code length symbols stop at 18
        }
    }
    // Skipped entries (codes 17/18) and never-written entries both stay at the
    // array's zero init, which is exactly how RFC 1951 fills the alphabets.
    let mut lit = [0u8; 288]; // the literal/length table's lengths
    let mut dist = [0u8; 30]; // the distance table's lengths
    lit[..hlit].copy_from_slice(&lens[..hlit]); // the front half belongs to literals
    dist[..hdist].copy_from_slice(&lens[hlit..hlit + hdist]); // the back half to distances
    Ok((lit, dist)) // hand both built tables to the caller
}

// One DEFLATE zlib stream -> raw PNG image bytes (rows pre-padded with their
// filter byte), exactly sized by the caller so every overrun is a clean Err.
fn inflate(src: &[u8], out: &mut [u8], expected: usize) -> Result<(), ()> {
    if src.len() < 2 { // a zlib stream can't be shorter than its header
        return Err(()); // corrupt
    }
    let cmf = src[0] as u32; // the first header byte
    let flg = src[1] as u32; // the second header byte
    if cmf & 0x0F != 8 || (cmf << 8 | flg) % 31 != 0 { // method 8 = deflate, header check = mod 31
        return Err(()); // not zlib data
    }
    let mut br = BitReader::new(src); // reader over the whole stream
    br.pos = 2; // already inside the 2-byte header, so skip it
    ensure_fixed(); // the fixed tables (built once, used when a block needs them)
    let mut written = 0usize; // bytes produced so far
    loop { // blocks until the final-block bit is set
        let last = br.bit()?; // one flag bit: 1 = last block
        let btype = br.bits(2)?; // two block-type bits
        match btype {
            0 => { // stored: raw bytes with a 2-byte check header
                let len = br.stored()? as usize; // read LEN (and its NLEN check)
                let bytes = br.take(len)?; // those many literal bytes
                if written + len > out.len() { // the caller chose the exact size
                    return Err(()); // overflow: corrupt
                }
                out[written..written + len].copy_from_slice(bytes); // copy the block verbatim
                written += len; // count it
            }
            1 => { // fixed Huffman tables
                let fixed = FIXED.get(); // the two prebuilt tables
                written = run_symbols(&mut br, &fixed[0], &fixed[1], out, written)?; // decode until end-of-block
            }
            2 => { // dynamic Huffman tables, defined by this block's header
                let (litl, distl) = read_dynamic(&mut br)?; // build both tables' code lengths
                let mut tables = [Huff::empty(), Huff::empty()]; // local table pair
                build(&mut tables[0], &litl); // literal/length table
                build(&mut tables[1], &distl); // distance table
                written = run_symbols(&mut br, &tables[0], &tables[1], out, written)?; // decode until end-of-block
            }
            _ => return Err(()), // btype 3 is illegal in RFC 1951
        }
        if last == 1 { // that block set the final-block flag
            break; // the stream is complete
        }
    }
    if written != expected { // the caller demands the exact decoded size
        return Err(()); // truncation or over-production: corrupt
    }
    Ok(())
}

// Decode one Huffman block: literals straight through, matches as LZ77
// copies, stop when the end-of-block symbol walks by. Returns total written.
fn run_symbols(br: &mut BitReader, lit: &Huff, dist: &Huff, out: &mut [u8], mut written: usize) -> Result<usize, ()> {
    loop { // until EOB
        let sym = decode(lit, br)?; // next literal/length symbol
        if sym < 256 { // a literal byte
            if written >= out.len() { // the caller chose the exact size
                return Err(()); // overflow: corrupt
            }
            out[written] = sym as u8; // store the byte
            written += 1; // one more byte out
        } else if sym == 256 { // end of block
            return Ok(written); // back to the caller for the next block
        } else { // a length/distance match to earlier output
            let li = (sym - 257) as usize; // index into the length tables
            if li >= 29 { // codes 286/287 are reserved
                return Err(()); // corrupt
            }
            let len = LENS[li] + br.bits(LEXT[li])?; // base length plus its extra bits
            let dsym = decode(dist, br)? as usize; // distance code
            if dsym >= 30 { // only 0..29 are legal
                return Err(()); // corrupt
            }
            let d = DSTS[dsym] + br.bits(DEXT[dsym])?; // base distance plus extra bits
            if d as usize > written || written + len as usize > out.len() { // must stay inside what we wrote
                return Err(()); // corrupt: references before the start or past the end
            }
            let from = written - d as usize; // where the match starts
            for i in 0..len as usize { // byte-wise copy handles overlapping windows
                out[written + i] = out[from + i]; // LZ77: matches may copy over themselves
            }
            written += len as usize; // the match produced length bytes
        }
    }
}

// -- PNG unfilter: reverse the 6 filter types in place over the strided rows --

// The inflated stream holds h rows of (1 filter byte + w*4 RGBA bytes). Each
// row's filter says how its bytes were mooshed by the previous row's values;
// this rewrites every row into real RGBA bytes, left to right, top to bottom.
fn unfilter(zs: &mut [u8], w: usize, h: usize) {
    let stride = w * 4 + 1; // bytes per stored row: one filter byte + image bytes
    let row = w * 4; // image bytes per row
    let mut prior = [0u8; VEHICLE_SRC_W as usize * 4]; // a stack copy of the previous row (widest source row)
    for r in 0..h { // rows top to bottom
        let base = r * stride; // this row's start inside the inflate buffer
        let f = zs[base]; // this row's filter byte, 0..4
        let mut x = 0usize; // byte column within the row's image bytes
        while x < row { // left to right, byte-wise
            let raw = zs[base + 1 + x] as u32; // the filtered byte the way the file wrote it
            let a = if x >= 4 { zs[base + 1 + x - 4] } else { 0 }; // reconstructed byte 4 to the left (already written this row)
            let b = prior[x]; // reconstructed byte right above (from the stack copy)
            let c = if x >= 4 { prior[x - 4] } else { 0 }; // reconstructed byte above-left
            let v = match f {
                0 => raw, // None: the byte is the answer
                1 => (raw + a as u32) & 0xFF, // Sub: add the left neighbour
                2 => (raw + b as u32) & 0xFF, // Up: add the upper neighbour
                3 => (raw + ((a as u32 + b as u32) >> 1)) & 0xFF, // Average: add floor((a + b) / 2)
                _ => { // Paeth, the famous predictor
                    let p = a as i32 + b as i32 - c as i32; // gradient estimate from three neighbours
                    let pa = (p - a as i32).abs(); // distance to the left guess
                    let pb = (p - b as i32).abs(); // distance to the upper guess
                    let pc = (p - c as i32).abs(); // distance to the upper-left guess
                    let pred = if pa <= pb && pa <= pc { a as u32 } else if pb <= pc { b as u32 } else { c as u32 }; // nearest of the three
                    (raw + pred) & 0xFF // Paeth: add the chosen neighbour
                }
            };
            zs[base + 1 + x] = v as u8; // write the reconstructed byte back over itself
            x += 1; // next byte of the row
        }
        prior[..row].copy_from_slice(&zs[base + 1..base + 1 + row]); // this row becomes the next row's "above"
    }
}

// -- chunk walker + the loading pipeline: PNG file -> final sprite buffer ----

// Decode one PNG file out of the scratch buffer into the inflate scratch area
// (both are raw byte buffers owned by the caller). IDAT payloads get packed to
// the FRONT of the file slice, so the zlib stream ends up one contiguous run.
// Returns the picture's own width/height; the decoded RGBA rows stay in `zs`
// laid out as (1 filter byte + 4 bytes per pixel) * h, already unfiltered.
fn decode_png(file: &mut [u8], zs: &mut [u8]) -> Result<(usize, usize), ()> {
    const SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // the PNG signature every file starts with
    if file.len() < 8 || file[..8] != SIG { // too short or the wrong magic
        return Err(()); // not a PNG at all
    }
    let mut at = 8; // chunk walk starts after the signature
    let (mut w, mut h) = (0usize, 0usize); // the IHDR's width/height
    let mut zlib_end = 0usize; // how many IDAT payload bytes are packed at the front
    loop { // chunk after chunk until IEND
        if at + 8 > file.len() { // the walk ran off the file with no IEND
            return Err(()); // corrupt
        }
        let len = u32::from_be_bytes([file[at], file[at + 1], file[at + 2], file[at + 3]]) as usize; // big-endian chunk length
        let kind = [file[at + 4], file[at + 5], file[at + 6], file[at + 7]]; // the chunk type word
        let data = at + 8; // where the chunk payload starts
        if data + len + 4 > file.len() { // payload plus CRC must fit
            return Err(()); // corrupt
        }
        match kind {
            [0x49, 0x48, 0x44, 0x52] => { // "IHDR" â€” the header chunk, always first in a real PNG
                if len != 13 { // IHDR is always exactly 13 bytes
                    return Err(()); // corrupt
                }
                w = u32::from_be_bytes([file[data], file[data + 1], file[data + 2], file[data + 3]]) as usize; // width in pixels
                h = u32::from_be_bytes([file[data + 4], file[data + 5], file[data + 6], file[data + 7]]) as usize; // height in pixels
                if file[data + 8] != 8 || file[data + 9] != 6 || file[data + 10] != 0 // 8-bit depth, colour type 6 (RGBA)
                    || file[data + 11] != 0 || file[data + 12] != 0 { // default compression/filter, no interlace
                    return Err(()); // only this one PNG flavour is supported
                }
                if w == 0 || h == 0 { // an empty picture is corrupt
                    return Err(());
                }
                if (w * 4 + 1) * h > ZSTRIDE_CAP { // bigger than the inflate scratch can hold
                    return Err(()); // refuse to overrun the scratch (Java would have OOM'd too)
                }
            }
            [0x49, 0x44, 0x41, 0x54] => { // "IDAT" â€” the compressed image data (PNG allows several IDATs)
                if w == 0 { // image data before the header is corrupt
                    return Err(());
                }
                file.copy_within(data..data + len, zlib_end); // move this payload to the front (memmove semantics)
                zlib_end += len; // it now sits right after the previous payload
            }
            [0x49, 0x45, 0x4E, 0x44] => { // "IEND" â€” end of image: stop reading chunks
                break;
            }
            _ => {} // ancillary chunks (the vehicles' 21,881-byte caBX block etc.) get skipped
        }
        at = data + len + 4; // past this chunk's payload and its CRC
    }
    if zlib_end == 0 { // no image data anywhere in the file
        return Err(()); // corrupt
    }
    let expected = (w * 4 + 1) * h; // exactly how many filtered bytes the image holds
    inflate(&file[..zlib_end], zs, expected)?; // zlib-decompress them into the inflate area
    unfilter(zs, w, h); // and turn the filtered rows into real RGBA bytes
    Ok((w, h))
}

// Java's resizeSharp: each destination pixel copies the nearest original one.
// `sx = x * srcW / dstW`, `sy = y * srcH / dstH`, plain integer division.
// src is the unfiltered strided decode; dst is 0xAARRGGBB words, row-major,
// written at `dst_stride` words per row. paint_bg: Some = flatten over one
// solid Java colour (vehicles/logs), None = keep alpha (Steve blends live).
fn resize_from_raw(src: &[u8], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize, dst_stride: usize, paint_bg: Option<u32>) {
    let src_stride = sw * 4 + 1; // the decode's row layout
    for y in 0..dh { // one destination row at a time
        let sy = y * sh / dh; // Java's nearest-neighbour row pick
        for x in 0..dw { // one destination pixel at a time
            let sx = x * sw / dw; // Java's nearest-neighbour column pick
            let at = sy * src_stride + 1 + sx * 4; // that pixel's first byte in the decode
            let b = src[at] as u32; // blue channel
            let g = src[at + 1] as u32; // green channel
            let r = src[at + 2] as u32; // red channel
            let a = src[at + 3] as u32; // alpha channel
            let px = (a << 24) | (r << 16) | (g << 8) | b; // assemble 0xAARRGGBB
            dst[y * dst_stride + x] = match paint_bg { // store it, flattened when asked
                Some(bg) => flatten(px, bg), // collision sprites bake their lane colour in
                None => px, // Steve keeps his alpha for the live blend
            };
        }
    }
}

// The same nearest-neighbour resize, but sampling from a stored row-major
// 0xAARRGGBB word buffer (the four log sources stay decoded in this form).
// Java re-runs resizeSharp from the original PImage on every log spawn; this
// is that spawn-time pick over the kept words, straight into a log slot.
fn resize_words(src: &[u32], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize, dst_stride: usize, paint_bg: Option<u32>) {
    for y in 0..dh { // one destination row at a time
        let sy = y * sh / dh; // Java's nearest-neighbour row pick
        for x in 0..dw { // one destination pixel at a time
            let sx = x * sw / dw; // Java's nearest-neighbour column pick
            let px = src[sy * sw + sx]; // the source word (alpha kept from load)
            dst[y * dst_stride + x] = match paint_bg { // flatten when asked
                Some(bg) => flatten(px, bg), // logs bake the water colour in
                None => px, // (not used today: logs always flatten)
            };
        }
    }
}

// Java draws every sprite with plain alpha blending onto the lane colour. For
// the collision sprites that blend is baked in here once: out = src over bg.
// The result's alpha becomes 255 so the draw-time blit is an opaque copy.
fn flatten(src: u32, bg: u32) -> u32 {
    let a = src >> 24; // source alpha, 0..255
    if a >= 255 { // fully opaque: the background never shows
        return 0xFF00_0000 | (src & 0x00FF_FFFF); // force the opaque alpha, keep RGB
    }
    if a == 0 { // fully transparent: pure background
        return bg | 0xFF00_0000; // opaque background colour
    }
    let dst_r = (bg >> 16) & 0xFF; // background red
    let dst_g = (bg >> 8) & 0xFF; // background green
    let dst_b = bg & 0xFF; // background blue
    let r = (src >> 16) & 0xFF; // source red
    let g = (src >> 8) & 0xFF; // source green
    let b = src & 0xFF; // source blue
    let inv = 255 - a; // the background's share
    let or = ((r * a + dst_r * inv + 127) / 255) as u32; // red: rounded src over bg
    let og = ((g * a + dst_g * inv + 127) / 255) as u32; // green: rounded src over bg
    let ob = ((b * a + dst_b * inv + 127) / 255) as u32; // blue: rounded src over bg
    0xFF00_0000 | (or << 16) | (og << 8) | ob // opaque blended pixel
}

// Build a 1-bit visibility mask (Java's makeHitboxImage logic): the bit at
// y * row_bits + x is 1 when that pixel's alpha >= minVisibleAlpha. Bits are
// packed LSB-first into u64 words; rows use row_bits (>= w) so window reads
// stay well inside the array, plus two pad words at the end for safety.
fn build_mask(px: &[u32], w: usize, h: usize, row_bits: usize, mask: &mut [u64]) {
    for m in mask.iter_mut() { // start from all-zeros
        *m = 0;
    }
    for y in 0..h { // one source row at a time
        let base = y * row_bits; // where this row's first bit lands
        let mut x = 0usize; // column cursor
        while x < w { // each pixel's bit
            if px[y * w + x] >> 24 >= MIN_VISIBLE_ALPHA { // Java's alpha threshold
                let idx = base + x; // the bit's global index
                mask[idx / 64] |= 1 << (idx % 64); // set that bit
            }
            x += 1;
        }
    }
}

// Read n_bits (<= 128) starting at bit index `start` from a 1-bit mask.
// Bits are numbered LSB-first inside each u64 word, word k holding bits
// k*64..k*64+64. With the pad words this can never index out of bounds.
#[inline]
fn mask_bits128(mask: &[u64], start: usize, n_bits: usize) -> u128 {
    let wi = start >> 6; // the first word index
    let sh = start & 63; // bit offset inside it
    let a = mask[wi] as u128; // bits starting at `start`
    let b = mask[wi + 1] as u128; // the next word
    let v = if sh == 0 { // window starts on a word boundary
        a | (b << 64) // two words line up perfectly
    } else {
        let c = mask[wi + 2] as u128; // the word after that (supplies bits past 128)
        (a >> sh) | (b << (64 - sh)) | (c << (128 - sh)) // three words stitch into one u128
    };
    v & ((1u128 << n_bits) - 1) // keep exactly n_bits of the window
}

// NOTE: when sh == 0, `b << 64` shifts a u128 by 64 (legal: u128 shifts up to 127);
// when sh > 64 - n_bits, the `c` word supplies the missing high bits. Both stay
// inside a 128-bit result, so one mask per call handles a whole row window.

// ---- the sprites: decoded once, then every draw is a plain memcpy ----

const VEH_COUNT: usize = 6; // 3 types x 2 directions, sprite index = type*2 + dir
const VEH_PX: usize = (VEHICLE_W * VEHICLE_H) as usize; // each resized vehicle is 420x140
const PLAYER_PX: usize = (PLAYER_W * PLAYER_H) as usize; // Steve resized: 74x135
const LOG_SLOT_PX: usize = LOG_SLOT_W as usize * LOG_SLOT_H as usize; // every log slot buffer: 320x112

#[inline(always)]
const fn mask_words(w: i32, h: i32) -> usize {
    ((w as usize * h as usize) + 127) / 64 + 2 // words for w*h bits, plus 2 pad words
}

static VEHICLE_DATA: Global<[u32; VEH_COUNT * VEH_PX]> = Global(UnsafeCell::new([0; VEH_COUNT * VEH_PX])); // the six flattened vehicle sprites
static VEHICLE_MASK: Global<[u64; VEH_COUNT * mask_words(VEHICLE_W, VEHICLE_H)]> = Global(UnsafeCell::new([0; VEH_COUNT * mask_words(VEHICLE_W, VEHICLE_H)])); // their 1-bit hitbox masks
static PLAYER_DATA: Global<[u32; PLAYER_PX]> = Global(UnsafeCell::new([0; PLAYER_PX])); // Steve's resized RGBA (alpha kept for the live blend)
static PLAYER_MASK: Global<[u64; mask_words(PLAYER_W, PLAYER_H)]> = Global(UnsafeCell::new([0; mask_words(PLAYER_W, PLAYER_H)])); // Steve's 1-bit hitbox mask
static STAGE_A: Global<[u32; VEH_PX]> = Global(UnsafeCell::new([0; VEH_PX])); // staging buffer for one vehicle resize
static LOG_BIRCH: Global<[u32; 1167 * 445]> = Global(UnsafeCell::new([0; 1167 * 445])); // birch_log.png: 1167x445
static LOG_MOSSY: Global<[u32; 1200 * 503]> = Global(UnsafeCell::new([0; 1200 * 503])); // mossy_log.png: 1200x503
static LOG_OAK: Global<[u32; 1213 * 511]> = Global(UnsafeCell::new([0; 1213 * 511])); // oak_log.png: 1213x511
static LOG_SHORT: Global<[u32; 835 * 391]> = Global(UnsafeCell::new([0; 835 * 391])); // short_log.png: 835x391
static LOG_SLOTS: Global<[[u32; LOG_SLOT_PX]; MAX_LOGS]> = Global(UnsafeCell::new([[0; LOG_SLOT_PX]; MAX_LOGS])); // each log slot's own resized copy

#[derive(Clone, Copy)]
struct LogSrc { // where one decoded log source lives + its own dimensions
    px: *mut u32, // start of the pixel words
    w: i32, // the picture's own width
    h: i32, // the picture's own height
}
static LOG_SRCS: Global<[LogSrc; 4]> = Global(UnsafeCell::new([LogSrc { px: null_mut(), w: 0, h: 0 }; 4])); // filled by load_images

static PATH_BUF: Global<[u16; 640]> = Global(UnsafeCell::new([0; 640])); // assembled asset paths live here

unsafe fn u16_len(mut p: *const u16) -> usize { // length of a NUL-terminated UTF-16 string
    let mut n = 0usize; // counter so far
    while *p != 0 { // stop at the terminator
        p = p.add(1); // walk the array
        n += 1;
    }
    n // the reported length
}

// Open "assets\images\<rel>", preferring the folder next to the exe and then
// falling back to the current directory (how `cargo run` builds find them).
unsafe fn open_asset(rel: *const u16) -> H {
    let buf = PATH_BUF.get(); // the shared path assembly buffer
    let bytes = GetModuleFileNameW(null_mut(), buf.as_mut_ptr(), 260) as usize; // the exe's full path lands there
    let mut dir = bytes; // cut the path back to its last '\'
    while dir > 0 && buf[dir - 1] != b'\\' as u16 { // cannot go past the root
        dir -= 1; // drop one character at a time
    }
    let mut n = dir; // the exe directory prefix stays first
    let parts = b"\\assets\\images\\"; // ASCII folder pieces to append
    for &c in parts { // widen each byte as we append
        buf[n] = c as u16;
        n += 1;
    }
    for i in 0..u16_len(rel) { // then the requested relative path
        buf[n] = *rel.add(i); // copy one UTF-16 unit
        n += 1;
    }
    buf[n] = 0; // terminate
    let h = CreateFileW(buf.as_ptr() as *const u16, GENERIC_READ, FILE_SHARE_READ, null_mut(), OPEN_EXISTING, 0, null_mut()); // open it
    if h != INVALID_HANDLE { // the exe-side copy is readable
        return h; // done
    }
    let mut m = 0usize; // fallback: run straight from the current directory
    for i in 0..u16_len(rel) { // copy again (cwd-relative this time)
        buf[m] = *rel.add(i); // the same relative path
        m += 1;
    }
    buf[m] = 0; // terminate
    CreateFileW(buf.as_ptr() as *const u16, GENERIC_READ, FILE_SHARE_READ, null_mut(), OPEN_EXISTING, 0, null_mut()) // second attempt
}

// Read one PNG into the scratch file area, decode and unfilter it into the
// scratch inflate area. Returns the picture's own (width, height).
unsafe fn decode_file(rel: *const u16) -> Result<(usize, usize), ()> {
    let file = open_asset(rel); // resolve the path and open
    if file == INVALID_HANDLE { // missing file: give up cleanly
        return Err(()); // (Java would throw on the same mistake)
    }
    let s = SCRATCH.get(); // the scratch buffers
    let mut got: u32 = 0; // bytes actually read
    ReadFile(file, s.file, FILE_CAP as u32, &mut got, null_mut()); // slurp the whole file
    CloseHandle(file); // done with the handle either way
    let file_bytes = core::slice::from_raw_parts_mut(s.file, got as usize); // decode over the read bytes
    let zs = core::slice::from_raw_parts_mut(s.zs, ZSTRIDE_CAP); // the full inflate area
    decode_png(file_bytes, zs) // walk chunks, inflate, unfilter
}

// Flatten a resize staging buffer into its final opaque sprite (road or river).
fn flatten_copy(src: &[u32], dst: &mut [u32], bg: u32) {
    for k in 0..src.len() { // one word per pixel
        dst[k] = flatten(src[k], bg); // blend the alpha away
    }
}

// Load all eleven PNGs and prepare every runnable sprite. Everything the game
// draws afterwards is a straight u32 copy, exactly like loading ahead in Java.
unsafe fn load_images() -> Result<(), ()> {
    ensure_scratch(); // one allocation for all of the decoding
    let vehicles = [wz!("car\\car_left.png"), wz!("car\\car_right.png"), wz!("truck\\truck_left.png"), wz!("truck\\truck_right.png"), wz!("bus\\bus_left.png"), wz!("bus\\bus_right.png")]; // Java's loadVehicles() list, in order
    for i in 0..VEH_COUNT { // each (type, direction) sprite
        let (w, h) = decode_file(vehicles[i])?; // decode into the inflate scratch
        debug_assert_eq!(w, VEHICLE_SRC_W as usize); // all six are the same size
        debug_assert_eq!(h, VEHICLE_SRC_H as usize); // (checked at load only)
        let stage = STAGE_A.get(); // the staging words, alpha preserved
        resize_from_raw(core::slice::from_raw_parts_mut(SCRATCH.get().zs, ZSTRIDE_CAP), w, h, stage, VEHICLE_W as usize, VEHICLE_H as usize, VEHICLE_W as usize, None); // Java resizeSharp -> stage
        let words = mask_words(VEHICLE_W, VEHICLE_H); // this mask's word count
        build_mask(stage, VEHICLE_W as usize, VEHICLE_H as usize, VEHICLE_W as usize, &mut VEHICLE_MASK.get()[i * words..(i + 1) * words]); // the hitbox bits (Java makeHitboxImage)
        flatten_copy(stage, &mut VEHICLE_DATA.get()[i * VEH_PX..(i + 1) * VEH_PX], ROAD_COLOR); // bake the road colour in
    }
    let (pw, ph) = decode_file(wz!("steve\\steve.png"))?; // Steve's own decode
    debug_assert_eq!((pw, ph), (PLAYER_SRC_W as usize, PLAYER_SRC_H as usize)); // confirmed only at load
    resize_from_raw(core::slice::from_raw_parts_mut(SCRATCH.get().zs, ZSTRIDE_CAP), pw, ph, PLAYER_DATA.get(), PLAYER_W as usize, PLAYER_H as usize, PLAYER_W as usize, None); // 74x135, alpha kept
    build_mask(PLAYER_DATA.get(), PLAYER_W as usize, PLAYER_H as usize, PLAYER_W as usize, PLAYER_MASK.get()); // his hitbox bits
    let log_names = [wz!("logs\\birch_log.png"), wz!("logs\\mossy_log.png"), wz!("logs\\oak_log.png"), wz!("logs\\short_log.png")]; // Java's logNames[] order
    let bufs = [LOG_BIRCH.get() as *mut [u32; 1167 * 445] as *mut u32, LOG_MOSSY.get() as *mut [u32; 1200 * 503] as *mut u32, LOG_OAK.get() as *mut [u32; 1213 * 511] as *mut u32, LOG_SHORT.get() as *mut [u32; 835 * 391] as *mut u32]; // where each source's words live
    for k in 0..4 { // the four log sources stay decoded at their real sizes
        let (w, h) = decode_file(log_names[k])?; // decode into the inflate scratch
        resize_from_raw(core::slice::from_raw_parts_mut(SCRATCH.get().zs, ZSTRIDE_CAP), w, h, core::slice::from_raw_parts_mut(bufs[k], (w * h) as usize), w, h, w, None); // unpack straight to the source buffer
        LOG_SRCS.get()[k] = LogSrc { px: bufs[k], w: w as i32, h: h as i32 }; // remember where it lives for the spawns
    }
    let s = SCRATCH.get(); // decode is done: release the scratch region
    if !s.file.is_null() { // it exists (ensure_scratch allocated it)
        VirtualFree(s.file as *mut c_void, 0, MEM_RELEASE); // return the pages to Windows
        s.file = null_mut(); // and forget the pointers
        s.zs = null_mut(); // both halves are gone at once (one region)
    }
    Ok(())
}

// ---- the game: Java's frogger.java state and update order, line for line ----

#[derive(Clone, Copy)]
struct Vehicle { // Java's class Vehicle: image/hitbox are shared sprites (see the data arrays)
    active: bool, // Java: non-null in the vehicles[] array
    x: i32, // Java: x â€” current left edge
    y: i32, // Java: y â€” fixed to its road lane (roadY + vehiclePadding)
    speed: i32, // Java: speed â€” signed! negative drives left
    tallness: i32, // Java: tallness â€” how high a jump clears it
    kind: i32, // which of the three vehicle images (car/truck/bus)
    dir: i32, // 0 = drives left, 1 = drives right (Java: image side)
    prev_x: i32, // this frame's old x, for the dirty-rect list
}

impl Vehicle {
    const BLANK: Vehicle = Vehicle { active: false, x: 0, y: 0, speed: 0, tallness: 0, kind: 0, dir: 0, prev_x: 0 }; // an empty slot
}

#[derive(Clone, Copy)]
struct LogSlot { // Java's class Log + the pixel buffer index it owns
    active: bool, // Java: non-null in the logs[] array
    x: i32, // Java: x â€” current left edge
    y: i32, // Java: y â€” centred vertically in the river
    w: i32, // Java: image.width â€” resized per spawn
    h: i32, // Java: image.height â€” random 84..112 per spawn
    speed: i32, // Java: speed â€” always positive, logs drift right
    prev_x: i32, // this frame's old x, for the dirty-rect list
}

impl LogSlot {
    const BLANK: LogSlot = LogSlot { active: false, x: 0, y: 0, w: 0, h: 0, speed: 0, prev_x: 0 }; // an empty slot
}

struct Game { // every Java frogger.java field, mirroring its game state
    // Player (Steve)
    player_x: i32, // Java: playerX
    player_y: i32, // Java: playerY
    moving_up: bool, // Java: movingUp
    moving_left: bool, // Java: movingLeft
    moving_down: bool, // Java: movingDown
    moving_right: bool, // Java: movingRight
    space_bar_down: bool, // Java: spaceBarDown â€” jump only once per press
    last_step_time: i64, // Java: lastStepTime
    // Jump
    jumping: bool, // Java: isJumping
    jump_frame: i32, // Java: jumpFrame â€” how many frames into the jump
    jump_distance_left: i32, // Java: jumpDistanceLeft
    jump_height_now: i32, // Java: currentJumpHeight â€” 0 = on the ground
    jump_start_time: i64, // Java: jumpStartTime
    // Vehicles
    vehicles: [Vehicle; MAX_VEHICLES], // Java: vehicles = new Vehicle[maxVehicles]
    next_road_one: Vehicle, // Java: nextRoadOneVehicle (speed pre-picked)
    next_road_two: Vehicle, // Java: nextRoadTwoVehicle
    last_road_one: Vehicle, // Java: lastRoadOneVehicle (the one ahead)
    last_road_two: Vehicle, // Java: lastRoadTwoVehicle
    has_last_road_one: bool, // Java: lastRoadOneVehicle != null
    has_last_road_two: bool, // Java: lastRoadTwoVehicle != null
    last_road_one_live: bool, // the "last" vehicle still moves in its slot
    last_road_two_live: bool, // (dropped vehicles keep a frozen Java x)
    last_road_one_slot: usize, // where the "last" vehicle lives in the slots
    last_road_two_slot: usize, // same, for road two
    next_road_one_spawn: i64, // Java: nextRoadOneSpawn
    next_road_two_spawn: i64, // Java: nextRoadTwoSpawn
    // Logs
    logs: [LogSlot; MAX_LOGS], // Java: logs = new Log[maxLogs]
    last_log: LogSlot, // Java: lastLog (the one ahead in the river)
    has_last_log: bool, // Java: lastLog != null
    last_log_live: bool, // the "last" log still moves in its slot
    last_log_slot: usize, // where the "last" log lives in the slots
    next_log_spawn: i64, // Java: nextLogSpawn
    next_log_speed: i32, // Java: nextLogSpeed (picked one spawn ahead)
    // Debug mode
    debug_mode: bool, // Java: debugMode â€” off until Shift+D
}

impl Game {
    const fn new() -> Self { // Java's field initialisers, all of them
        Game { // construct the record
            player_x: 0, // Java: int playerX = 0
            player_y: 0, // Java: int playerY = 0
            moving_up: false, // Java: boolean movingUp = false
            moving_left: false, // Java: boolean movingLeft = false
            moving_down: false, // Java: boolean movingDown = false
            moving_right: false, // Java: boolean movingRight = false
            space_bar_down: false, // Java: boolean spaceBarDown = false
            last_step_time: 0, // Java: int lastStepTime = 0
            jumping: false, // Java: boolean isJumping = false
            jump_frame: 0, // Java: int jumpFrame = 0
            jump_distance_left: 0, // Java: int jumpDistanceLeft = 0
            jump_height_now: 0, // Java: int currentJumpHeight = 0 (0 = grounded)
            jump_start_time: -JUMP_COOLDOWN, // Java: jumpStartTime = -jumpCooldown (jump allowed immediately)
            vehicles: [Vehicle::BLANK; MAX_VEHICLES], // Java: new Vehicle[maxVehicles]
            next_road_one: Vehicle::BLANK, // Java: null until setup()
            next_road_two: Vehicle::BLANK, // Java: null until setup()
            last_road_one: Vehicle::BLANK, // Java: lastRoadOneVehicle = null
            last_road_two: Vehicle::BLANK, // Java: lastRoadTwoVehicle = null
            has_last_road_one: false, // Java: null, so hasRoomFor short-circuits
            has_last_road_two: false, // same for road two
            last_road_one_live: false, // nothing moves in the slot yet
            last_road_two_live: false, // nothing moves in the slot yet
            last_road_one_slot: 0, // unused while not live
            last_road_two_slot: 0, // unused while not live
            next_road_one_spawn: 0, // Java: nextRoadOneSpawn = 0, set in setup()
            next_road_two_spawn: 0, // Java: nextRoadTwoSpawn = 0, set in setup()
            logs: [LogSlot::BLANK; MAX_LOGS], // Java: new Log[maxLogs]
            last_log: LogSlot::BLANK, // Java: lastLog = null
            has_last_log: false, // Java: null, so hasRoomForLog short-circuits
            last_log_live: false, // nothing moves in the slot yet
            last_log_slot: 0, // unused while not live
            next_log_spawn: 0, // Java: nextLogSpawn = 0, set in setup()
            next_log_speed: 0, // Java: nextLogSpeed = 0, set in setup()
            debug_mode: false, // Java: boolean debugMode = false
        }
    }
}

static GAME: Global<Game> = Global(UnsafeCell::new(SEE_GAME)); // the one game instance
const SEE_GAME: Game = Game::new(); // the constant seed the field starts as

// Java's placePlayerAtStart(): centred horizontally, resting just below Road Two.
fn place_player_at_start(g: &mut Game) {
    g.player_x = (WINDOW_W - PLAYER_W) / 2; // Java: (width - playerImage.width) / 2
    g.player_y = WINDOW_H - PLAYER_H - PLAYER_START_GAP; // Java: height - playerImage.height - playerStartGap
}

// Java's makeVehicle(roadY, isMovingRight), minus the image lookup (the sprite
// index encodes type + direction, and the vehicles' size never varies).
fn make_vehicle(road_y: i32, is_moving_right: bool) -> Vehicle {
    let kind = rng_i32(0, 3); // Java: (int) random(vehicleNames.length) -> 0..2
    let speed = rng_i32(MIN_VEHICLE_SPEEDS[kind as usize], MAX_VEHICLE_SPEEDS[kind as usize] + 1); // Java: (int) random(minSpeeds[type], maxSpeeds[type] + 1)
    let dir = if is_moving_right { 1 } else { 0 }; // sprite index half (Java picks the right/left image)
    Vehicle {
        active: false, // not in the slots array until addVehicle stores it
        x: if is_moving_right { -VEHICLE_W } else { WINDOW_W }, // Java: -image.width, or width itself
        y: road_y + VEHICLE_PADDING, // Java: vehicle.y = roadY + vehiclePadding
        speed: if is_moving_right { speed } else { -speed }, // Java negates the speed to drive left
        tallness: VEHICLE_TALLNESS[kind as usize], // Java: vehicleTallness[type]
        kind, // the sprite's colour/type (car, truck, bus)
        dir, // drives left or right
        prev_x: 0, // set when the vehicle is added and moves later
    }
}

// Java's getDistanceFromEdge: how far the vehicle's BACK is from the edge it
// drove in from (that's the side a newer vehicle would crowd).
fn get_distance_from_edge(v: Vehicle) -> i32 {
    if v.speed > 0 { // drove in from the left edge
        v.x // the back is the vehicle's left side
    } else { // drove in from the right edge
        WINDOW_W - (v.x + VEHICLE_W) // the back is its right side, mirrored
    }
}

// Java's hasRoomFor(newVehicle, vehicleAhead): would a NEWER vehicle entering
// now still stay vehicleGap behind the one ahead once that one exits?
fn has_room_for(new_v: Vehicle, ahead: Option<Vehicle>) -> bool {
    let Some(a) = ahead else { return true; }; // Java: vehicleAhead == null -> true
    let dist = get_distance_from_edge(a); // where the trap starts
    if dist > WINDOW_W { // the one ahead is already fully off screen
        return true; // Java: getDistanceFromEdge(vehicleAhead) > width -> true
    }
    if dist < VEHICLE_GAP { // it's still driving in the window's edge strip
        return false; // Java: distanceAhead < vehicleGap -> false
    }
    let speed_ahead = a.speed.abs(); // Java: abs(vehicleAhead.speed)
    let frames_left = (WINDOW_W - dist) / speed_ahead; // Java's frames until the vehicle ahead exits
    let gained = (new_v.speed.abs() - speed_ahead) * frames_left; // how much closer a new vehicle would draw
    dist - gained >= VEHICLE_GAP // the final gap must still hold
}

// Java's addVehicle: the first empty slot, or silently dropped (Java quirk).
fn add_vehicle(g: &mut Game, v: Vehicle) -> Option<usize> {
    let mut v = v; // Java stores the live object; the port's "non-null" is the active bit
    v.active = true; // any vehicle in a slot is on the road (makeVehicle pre-builds inactive ones)
    for i in 0..MAX_VEHICLES { // Java: for (int index = 0; ...)
        if !g.vehicles[i].active { // Java: vehicles[index] == null
            g.vehicles[i] = v; // Java: vehicles[index] = vehicle
            return Some(i); // slot found
        }
    }
    None // Java: the loop just falls through and the vehicle vanishes
}

// Java's hasRoomForLog(speed), with lastLog's live position.
fn has_room_for_log(g: &Game, speed: i32) -> bool {
    if !g.has_last_log { // Java: lastLog == null
        return true; // first log of the game
    }
    let last = g.last_log; // a copy of the last spawned log
    if last.x > WINDOW_W { // it already floated off the screen
        return true; // Java: lastLog.x > width -> true
    }
    if last.x < LOG_GAP { // still floating in from the edge
        return false; // Java: lastLog.x < logGap -> false
    }
    let frames_left = (WINDOW_W - last.x) / last.speed; // Java: frames until the last log exits
    let gained = (speed - last.speed) * frames_left; // Java: how much closer a new log draws
    last.x - gained >= LOG_GAP // the final gap must still hold
}

// Java's addLog: the first empty slot, or silently dropped (Java quirk).
fn add_log(g: &mut Game, l: LogSlot) -> Option<usize> {
    for i in 0..MAX_LOGS { // Java: for (int index = 0; ...)
        if !g.logs[i].active { // Java: logs[index] == null
            g.logs[i] = l; // Java: logs[index] = log
            return Some(i); // slot found
        }
    }
    None // Java: the loop just falls through and the log vanishes
}

// ---- the frame update: Java's updateGame(), one function per Java method ----

// Push one dirty rectangle (Java repaints everything; we repaint exact areas).
static DIRTY: Global<[Rect; 64]> = Global(UnsafeCell::new([Rect::EMPTY; 64])); // the frame's rectangles to repaint
static DIRTY_N: Global<usize> = Global(UnsafeCell::new(0)); // how many of those are valid

fn push_dirty(r: Rect) { // register a rectangle for the next repaint
    let r = r.clamp_client(); // nothing outside the window can ever show
    if r.is_empty() { // an empty rect paints nothing
        return; // skip it entirely
    }
    let n = *DIRTY_N.get(); // current fill level
    if n < 64 { // room for one more (worst case is well under)
        DIRTY.get()[n] = r; // store it
        *DIRTY_N.get() = n + 1; // count it
    }
    // over the cap the rect is dropped; the warps always cover those pixels
}

// Java's updateGame(): the exact same order every frame.
fn update_game(g: &mut Game) {
    let player_before = draw_box(g.player_x, g.player_y, g.jump_height_now); // Steve's painted box, before the update
    if g.jumping { // Java: if (isJumping) updateJump(); else keepWalking();
        update_jump(g);
    } else {
        keep_walking(g);
    }
    spawn_vehicles(g); // Java: spawnVehicles();
    move_vehicles(g); // Java: moveVehicles();
    spawn_logs(g); // Java: spawnLogs();
    move_logs(g); // Java: moveLogs();
    check_collisions(g); // Java: checkCollisions();
    if g.jump_height_now == 0 { // Java: water can't get a player in the air
        check_river(g); // Java: checkRiver();
    }
    let player_after = draw_box(g.player_x, g.player_y, g.jump_height_now); // Steve's painted box, after the update
    push_dirty(player_before); // repaint both ends of every Steve move,
    push_dirty(player_after); // one rect pair per frame, exactly like a full redraw
}

// Java's drawPlayer() maths: the painted box grows with the jump height.
fn draw_box(px: i32, py: i32, height_now: i32) -> Rect {
    let growth = JUMP_GROWTH * height_now / JUMP_HEIGHT; // Java: growth = jumpGrowth * currentJumpHeight / jumpHeight
    let dw = PLAYER_W + growth; // Java: drawWidth = playerImage.width + growth
    let dh = PLAYER_H + growth * 2; // Java: drawHeight = playerImage.height + growth * 2
    let dx = px - (dw - PLAYER_W) / 2; // Java: drawX = playerX - (drawWidth - playerImage.width) / 2
    let dy = py - (dh - PLAYER_H) / 2; // Java: drawY = playerY - (drawHeight - playerImage.height) / 2
    Rect::from_xywh(dx, dy, dw, dh) // the box Steve gets painted into
}

// Java's stepPlayer(stepX, stepY): one step, never in mid-air.
fn step_player(g: &mut Game, step_x: i32, step_y: i32) {
    if g.jumping { // Java: if (isJumping) return; â€” the jump moves Steve
        return;
    }
    g.player_x += step_x; // Java: playerX += stepX
    g.player_y += step_y; // Java: playerY += stepY
    check_edges(g); // Java: checkEdges();
    g.last_step_time = millis(); // Java: lastStepTime = millis()
}

// Java's keepWalking(): another step per frame while a key is held.
fn keep_walking(g: &mut Game) {
    let held = g.moving_up || g.moving_left || g.moving_down || g.moving_right; // Java: isHoldingKey
    if held && millis() - g.last_step_time >= 0 { // Java: stepDelay == 0, so every frame counts (this also re-steps the press's own frame: the double step)
        move_player(g, PLAYER_SPEED); // Java: movePlayer(playerSpeed)
        g.last_step_time = millis(); // Java: lastStepTime = millis()
    }
}

// Java's movePlayer(distance): every held axis takes that many pixels.
fn move_player(g: &mut Game, distance: i32) {
    if g.moving_up { // Java: if (movingUp) playerY -= distance
        g.player_y -= distance;
    }
    if g.moving_left { // Java: if (movingLeft) playerX -= distance
        g.player_x -= distance;
    }
    if g.moving_down { // Java: if (movingDown) playerY += distance
        g.player_y += distance;
    }
    if g.moving_right { // Java: if (movingRight) playerX += distance
        g.player_x += distance;
    }
    check_edges(g); // Java: checkEdges(); â€” the wall wrap runs here every time
}

// Java's jumpPlayer(): start a jump if the cooldown has run out.
fn jump_player(g: &mut Game) {
    if g.jumping || millis() - g.jump_start_time < JUMP_COOLDOWN { // Java: if (isJumping || millis() - jumpStartTime < jumpCooldown) return;
        return;
    }
    g.jumping = true; // Java: isJumping = true
    g.jump_start_time = millis(); // Java: jumpStartTime = millis()
    g.jump_frame = 0; // Java: jumpFrame = 0
    g.jump_distance_left = JUMP_LENGTH; // Java: jumpDistanceLeft = jumpLength
}

// Java's updateJump(): split the distance left over the frames left, no
// rounding drift, an arc of 0..jumpHeight..0, and land on the last frame.
fn update_jump(g: &mut Game) {
    g.jump_frame += 1; // Java: jumpFrame++
    let frames_left = JUMP_FRAMES - g.jump_frame + 1; // Java: framesLeft = jumpFrames - jumpFrame + 1
    let step = g.jump_distance_left / frames_left; // Java: step = jumpDistanceLeft / framesLeft (integer)
    g.jump_distance_left -= step; // Java: jumpDistanceLeft -= step (consumed whether or not a key is held)
    let diagonal = (g.moving_up || g.moving_down) && (g.moving_left || g.moving_right); // Java: isDiagonal
    let dist = if diagonal {
        round_half_up(step as f64 / SQRT_2) // Java: round(step / sqrt(2))
    } else {
        step // Java: straight step
    };
    move_player(g, dist); // Java: movePlayer(...); applies BOTH held axes
    g.jump_height_now = 4 * JUMP_HEIGHT * g.jump_frame * (JUMP_FRAMES - g.jump_frame) / (JUMP_FRAMES * JUMP_FRAMES); // Java: 4*jumpHeight*jumpFrame*(jumpFrames-jumpFrame)/(jumpFrames*jumpFrames)
    if g.jump_frame >= JUMP_FRAMES { // Java: land once all jump frames are used
        g.jumping = false; // Java: isJumping = false
        g.jump_height_now = 0; // Java: currentJumpHeight = 0
    }
}

// Java's spawnVehicles(): spawn into the two roads when there's room.
fn spawn_vehicles(g: &mut Game) {
    let now = millis(); // one clock read per call (Java calls millis() twice, harmless)
    // Road One: drives left to right
    if now >= g.next_road_one_spawn && has_room_for(g.next_road_one, Some(g.last_road_one).filter(|_| g.has_last_road_one)) {
        let mut spawn = g.next_road_one; // the pre-picked vehicle becomes real
        spawn.prev_x = spawn.x; // its dirty rect starts where it enters
        match add_vehicle(g, spawn) { // Java: addVehicle(nextRoadOneVehicle)
            Some(i) => { // it landed in a slot
                g.last_road_one = spawn; // Java: lastRoadOneVehicle = nextRoadOneVehicle
                g.has_last_road_one = true; // later spawns see it as the one ahead
                g.last_road_one_live = true; // and its x follows the slot as it moves
                g.last_road_one_slot = i; // remember which slot that is
            }
            None => { // no slot free: Java drops it silently
                g.last_road_one = spawn; // Java still advances its last pointer (frozen x quirk, mirrored)
                g.has_last_road_one = true; // and still spawns against it later
                g.last_road_one_live = false; // it never moves (it's not in any slot)
            }
        }
        g.next_road_one = make_vehicle(ROAD_ONE_Y, true); // Java: nextRoadOneVehicle = makeVehicle(roadOneY, true)
        g.next_road_one_spawn = now + rng_i32(MIN_SPAWN_DELAY, MAX_SPAWN_DELAY) as i64; // Java: millis() + (int) random(minSpawnDelay, maxSpawnDelay)
        push_dirty(Rect::from_xywh(spawn.x, spawn.y, VEHICLE_W, VEHICLE_H)); // its spawn rect (off screen, so usually empty)
    }
    // Road Two: drives right to left
    if now >= g.next_road_two_spawn && has_room_for(g.next_road_two, Some(g.last_road_two).filter(|_| g.has_last_road_two)) {
        let mut spawn = g.next_road_two; // same dance, other road
        spawn.prev_x = spawn.x;
        match add_vehicle(g, spawn) {
            Some(i) => {
                g.last_road_two = spawn;
                g.has_last_road_two = true;
                g.last_road_two_live = true;
                g.last_road_two_slot = i;
            }
            None => {
                g.last_road_two = spawn;
                g.has_last_road_two = true;
                g.last_road_two_live = false;
            }
        }
        g.next_road_two = make_vehicle(ROAD_TWO_Y, false); // Java: makeVehicle(roadTwoY, false)
        g.next_road_two_spawn = now + rng_i32(MIN_SPAWN_DELAY, MAX_SPAWN_DELAY) as i64; // Java: millis() + (int) random(...)
        push_dirty(Rect::from_xywh(spawn.x, spawn.y, VEHICLE_W, VEHICLE_H)); // off screen -> empty
    }
}

// Java's moveVehicles(): advance every active vehicle, free finished slots.
fn move_vehicles(g: &mut Game) {
    for i in 0..MAX_VEHICLES { // Java: for (int index = 0; ...)
        let mut v = g.vehicles[i]; // work on a copy like Java's local
        if !v.active { // Java: if (vehicle == null) continue;
            continue;
        }
        v.prev_x = v.x; // the frame's start position (Java's current x before the step)
        let nx = v.x + v.speed; // Java: vehicle.x += vehicle.speed
        push_dirty(Rect::from_xywh(v.x, v.y, VEHICLE_W, VEHICLE_H)); // repaint where it was,
        push_dirty(Rect::from_xywh(nx, v.y, VEHICLE_W, VEHICLE_H)); // and where it lands now
        let now_past_right = nx > WINDOW_W; // Java: isPastRightEdge = vehicle.x > width
        let now_past_left = nx + VEHICLE_W < 0; // Java: isPastLeftEdge = vehicle.x + image.width < 0
        v.x = nx; // commit the move
        if now_past_right || now_past_left { // Java: if (isPastRightEdge || isPastLeftEdge)
            v.active = false; // Java: vehicles[index] = null (free the slot)
        }
        g.vehicles[i] = v; // store the moved/possibly freed slot
    }
    if g.last_road_one_live && g.vehicles[g.last_road_one_slot].active { // a "last" vehicle still on screen follows its slot's move
        g.last_road_one.x = g.vehicles[g.last_road_one_slot].x; // Java: same object in both places
    } else if g.last_road_one_live { // its slot just got freed
        g.last_road_one_live = false; // from here Java's object x is frozen
    }
    if g.last_road_two_live && g.vehicles[g.last_road_two_slot].active { // same for road two
        g.last_road_two.x = g.vehicles[g.last_road_two_slot].x;
    } else if g.last_road_two_live {
        g.last_road_two_live = false;
    }
}

// Java's spawnLogs(): the river flow, one log at a time.
fn spawn_logs(g: &mut Game) {
    let now = millis(); // Java: millis() gate
    if now >= g.next_log_spawn && has_room_for_log(g, g.next_log_speed) {
        let kind = rng_i32(0, 4); // Java: (int) random(logNames.length) -> 0..3
        let h = rng_i32(MIN_LOG_H, MAX_LOG_H + 1); // Java: (int) random(minLogHeight, maxLogHeight + 1) -> 84..112
        let (sw, sh) = (LOG_SRCS.get()[kind as usize].w, LOG_SRCS.get()[kind as usize].h); // the source picture's own size
        let w = sw * h / sh; // Java resizeSharp's newWidth = width * newHeight / height
        let mut slot = LogSlot { // build the log exactly like Java's new Log()
            active: true, // (it's real once a slot accepts it)
            x: -w, // Java: log.x = -log.image.width
            y: RIVER_Y + (RIVER_HEIGHT - h) / 2, // Java: riverY + (riverHeight - logHeight) / 2, centred
            w, // the resized width
            h, // the random height
            speed: g.next_log_speed, // Java: log.speed = nextLogSpeed
            prev_x: -w, // dirty list baseline (never actually drawn there)
        };
        match add_log(g, slot) { // Java: addLog(log)
            Some(i) => { // the slot owns a resize of the source now
                let slice = unsafe { // the slot's source words (image data written once at load)
                    core::slice::from_raw_parts_mut(LOG_SRCS.get()[kind as usize].px, (sw * sh) as usize) // build the slice from the raw pointer
                }; // the source's pixel words
                resize_words(slice, sw as usize, sh as usize, &mut LOG_SLOTS.get()[i], w as usize, h as usize, LOG_SLOT_W as usize, Some(RIVER_COLOR)); // resizeSharp + bake the water colour
                slot = g.logs[i]; // the stored slot (same values)
                g.last_log = slot; // Java: lastLog = log
                g.has_last_log = true;
                g.last_log_live = true; // its x follows the slot as it drifts
                g.last_log_slot = i;
            }
            None => { // dropped like Java does
                g.last_log = slot; // Java still advances lastLog (frozen x quirk, mirrored)
                g.has_last_log = true;
                g.last_log_live = false;
            }
        }
        g.next_log_spawn = now + LOG_SPAWN_DELAY; // Java: millis() + logSpawnDelay
        g.next_log_speed = rng_i32(MIN_LOG_SPEED, MAX_LOG_SPEED + 1); // Java: (int) random(minLogSpeed, maxLogSpeed + 1)
        push_dirty(Rect::from_xywh(slot.x, slot.y, slot.w, slot.h)); // its spawn rect (off screen, so empty)
    }
}

// Java's moveLogs(): drift every log right, free finished slots.
fn move_logs(g: &mut Game) {
    for i in 0..MAX_LOGS { // Java: for (int index = 0; ...)
        let mut l = g.logs[i]; // like Java's local copy
        if !l.active { // Java: if (log == null) continue;
            continue;
        }
        l.prev_x = l.x; // frame start position
        let nx = l.x + l.speed; // Java: log.x += log.speed
        push_dirty(Rect::from_xywh(l.x, l.y, l.w, l.h)); // repaint where it was,
        push_dirty(Rect::from_xywh(nx, l.y, l.w, l.h)); // and where it lands now
        l.x = nx; // commit the drift
        if nx > WINDOW_W { // Java: if (log.x > width)
            l.active = false; // Java: logs[index] = null (free the slot)
        }
        g.logs[i] = l; // store the moved/possibly freed slot
    }
    if g.last_log_live && g.logs[g.last_log_slot].active { // the "last" log, still on screen, follows its slot
        g.last_log.x = g.logs[g.last_log_slot].x; // Java: same object in both places
    } else if g.last_log_live { // its slot just floated off
        g.last_log_live = false; // from here Java's object x is frozen
    }
}

// Java's checkCollisions(): jump over trucks, or get sent back to the start.
fn check_collisions(g: &mut Game) {
    for i in 0..MAX_VEHICLES { // Java: for (Vehicle vehicle : vehicles)
        let v = g.vehicles[i]; // like Java's loop variable
        if !v.active { // Java: if (vehicle == null) continue;
            continue;
        }
        let over = g.jump_height_now > v.tallness; // Java: isJumpingOver = currentJumpHeight > vehicle.tallness
        if !over && is_player_hitting(g, v) { // finally the pixel test
            place_player_at_start(g); // Java: placePlayerAtStart(); return; â€” and stops checking
            return;
        }
    }
}

// Java's isPlayerHitting(vehicle): AABB quick check, then a real pixel scan
// over the player box. The pixel scan becomes a word-AND over two 1-bit
// visibility masks (exactly the "screen pixel where both sprites show" test).
fn is_player_hitting(g: &Game, v: Vehicle) -> bool {
    let touch = g.player_x < v.x + VEHICLE_W // Java's boxesTouch, unrolling the shapes
        && g.player_x + PLAYER_W > v.x
        && g.player_y < v.y + VEHICLE_H
        && g.player_y + PLAYER_H > v.y;
    if !touch { // Java: if (!boxesTouch) return false;
        return false;
    }
    let x0 = g.player_x.max(v.x); // overlap left
    let x1 = (g.player_x + PLAYER_W).min(v.x + VEHICLE_W); // overlap right
    let y0 = g.player_y.max(v.y); // overlap top
    let y1 = (g.player_y + PLAYER_H).min(v.y + VEHICLE_H); // overlap bottom
    let words = mask_words(VEHICLE_W, VEHICLE_H); // the shared vehicle stride
    let vi = (v.kind * 2 + v.dir) as usize; // Java: which vehicle image this slot uses
    let vis = VEHICLE_MASK.get(); // the six packed masks
    masks_touch(PLAYER_MASK.get(), &vis[vi * words..(vi + 1) * words], g.player_x, g.player_y, v.x, v.y, x0, y0, x1, y1) // the word-AND pixel scan
}

// The overlap scan itself: for every row of the player/vehicle overlap, AND
// the two 1-bit visibility windows and look for a shared visible pixel.
// (Java scans all of Steve's screen pixels; the AND is that exact test.)
fn masks_touch(p_mask: &[u64], v_mask: &[u64], px: i32, py: i32, vx: i32, vy: i32, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    for y in y0..y1 { // one row of both sprites at a time
        let bits = (x1 - x0) as usize; // exactly the overlap's columns
        let p = mask_bits128(p_mask, (y - py) as usize * PLAYER_W as usize + (x0 - px) as usize, bits); // Steve's visible pixels
        let v = mask_bits128(v_mask, (y - vy) as usize * VEHICLE_W as usize + (x0 - vx) as usize, bits); // the vehicle's visible pixels
        if p & v != 0 { // some pixel shows in BOTH (Java returns true here)
            return true; // hit
        }
    }
    false // Java: return false; â€” every overlapping pixel was empty
}

// Java's checkRiver(): centre-of-player test against the water and the logs.
fn check_river(g: &mut Game) {
    let cx = g.player_x + PLAYER_W / 2; // Java: playerCenterX = playerX + playerImage.width / 2
    let cy = g.player_y + PLAYER_H / 2; // Java: playerCenterY
    let in_river = cy >= RIVER_Y && cy < RIVER_Y + RIVER_HEIGHT; // Java: isInRiver
    if !in_river { // Java: if (!isInRiver) return;
        return;
    }
    for i in 0..MAX_LOGS { // Java: for (Log log : logs)
        let l = g.logs[i];
        if l.active && cx >= l.x && cx < l.x + l.w && cy >= l.y && cy < l.y + l.h { // Java: isOnLog(log, ...)
            g.player_x += l.speed; // Java: playerX += log.speed â€” float along
            return;
        }
    }
    place_player_at_start(g); // Java: placePlayerAtStart(); â€” no log under Steve
}

// Java's checkEdges(): wrap around all four window walls.
fn check_edges(g: &mut Game) {
    let right = WINDOW_W - PLAYER_W; // Java: rightEdge = width - playerImage.width (1826)
    let bottom = WINDOW_H - PLAYER_H; // Java: bottomEdge = height - playerImage.height (865)
    if g.player_x < 0 { // Java: left wall wraps to the right edge
        g.player_x = right;
    }
    if g.player_x > right { // Java: right wall wraps to the left edge
        g.player_x = 0;
    }
    if g.player_y < 0 { // Java: top wall wraps to the bottom edge
        g.player_y = bottom;
    }
    if g.player_y > bottom { // Java: bottom wall wraps to the top edge
        g.player_y = 0;
    }
}

// ---- the renderer: dirty-rect composition into a 32bpp DIB back buffer ----
//
// Java's drawGame() paints the whole window every frame: background, roads,
// river, logs, vehicles, Steve, hitboxes, FPS text. We keep a full-frame DIB
// and repaint only the exact rectangles that changed â€” the bands get filled,
// all sprites that touch the rect re-blit in the Java draw order, then the
// debug overlays, then (once, always) the FPS text via GDI.

static DIB: Global<*mut u32> = Global(UnsafeCell::new(null_mut())); // the back buffer's pixels (BGRA, 0xAARRGGBB as u32)
static BACK_DC: Global<H> = Global(UnsafeCell::new(null_mut())); // memory dc the DIB bitmap is selected into
static GAME_FONT: Global<H> = Global(UnsafeCell::new(null_mut())); // the FPS counter's font

// The u32 pixels behind DIB at (x, y), written as 0xAARRGGBB (BGRA bytes).
#[inline(always)]
unsafe fn dib_at(x: i32, y: i32) -> *mut u32 {
    DIB.get().add((y as usize) * WINDOW_W as usize + x as usize) // y * width + x
}

// Fill [x0, x1) of one row with a colour.
unsafe fn fill_span(x0: i32, x1: i32, y: i32, color: u32) {
    let mut p = dib_at(x0, y); // first pixel
    let end = dib_at(x1, y); // past the last one (writes go to end-1)
    while p < end { // straight run
        *p = color; // Java's fill(c) pixel
        p = p.add(1); // next pixel
    }
}

// Fill the whole rectangle with a colour (Java's background(c)).
unsafe fn fill_rect(r: Rect, color: u32) {
    let mut y = r.y0; // first row (caller passes an already-clamped, non-empty rect)
    while y < r.y1 { // row after row
        fill_span(r.x0, r.x1, y, color); // one span
        y += 1;
    }
}

// Java's image(img, x, y) with an already-flattened (opaque) sprite: blit any
// intersecting part of it into the current dirty rect. src_stride is the
// buffer's row width in words (sprite data is stored tightly; slot buffers
// are 320 wide with a smaller live area).
unsafe fn blit_flat(src: *const u32, src_stride: i32, sw: i32, sh: i32, dx: i32, dy: i32, r: Rect) {
    let x0 = dx.max(r.x0); // clip to the dirty rect (Java's image() doesn't clip; we do, so exactly the rect composes)
    let x1 = (dx + sw).min(r.x1);
    let y0 = dy.max(r.y0);
    let y1 = (dy + sh).min(r.y1);
    if x0 >= x1 || y0 >= y1 { // the whole sprite sits beside the rect on this axis: nothing to draw
        return; // (the count below underflows otherwise; a sprite can be far from a small rect)
    }
    let mut sy = y0 - dy; // row index inside the sprite
    let mut ty = y0; // destination row
    while ty < y1 { // row by row
        let srow = src.add((sy * src_stride) as usize); // the source row's start
        let drow = dib_at(x0, ty); // the destination row's start
        let n = (x1 - x0) as usize; // pixels to copy (Java's image() is a plain copy
        let mut k = 0usize;
        while k < n {
            *drow.add(k) = *srow.add((x0 - dx) as usize + k); // already flattened, opaque, no blending
            k += 1;
        }
        sy += 1; // next source row
        ty += 1;
    }
}

// Java's image(playerImage, drawX, drawY, drawW, drawH): read the DIB under
// each destination pixel, blend Steve's RGBA on top, write it back. Sampling
// is nearest like Processing's noSmooth: (pixel - origin) * source / dest.
unsafe fn blit_player(r: Rect) {
    let g = GAME.get(); // the one game state
    let growth = JUMP_GROWTH * g.jump_height_now / JUMP_HEIGHT; // Java: growth (how much bigger Steve looks)
    let dw = PLAYER_W + growth; // Java: drawWidth
    let dh = PLAYER_H + growth * 2; // Java: drawHeight
    let dx = g.player_x - (dw - PLAYER_W) / 2; // Java: drawX â€” grown from the centre
    let dy = g.player_y - (dh - PLAYER_H) / 2; // Java: drawY
    let player = PLAYER_DATA.get(); // Steve's RGBA words
    let x0 = dx.max(r.x0); // clip to the dirty rect
    let x1 = (dx + dw).min(r.x1);
    let y0 = dy.max(r.y0);
    let y1 = (dy + dh).min(r.y1);
    let mut ty = y0; // first destination row
    while ty < y1 { // row by row
        let sy = (ty - dy) * PLAYER_H / dh; // Java's nearest row pick (truncated)
        let mut tx = x0; // first destination column
        while tx < x1 { // pixel by pixel
            let sx = (tx - dx) * PLAYER_W / dw; // Java's nearest column pick
            let src = player[(sy as usize) * PLAYER_W as usize + sx as usize]; // Steve's source pixel
            let a = src >> 24; // 0..255
            if a >= 255 { // opaque: straight copy
                *dib_at(tx, ty) = src; // Java writes the pixel verbatim
            } else if a > 0 { // partial: blend over whatever is already there
                let dst = *dib_at(tx, ty); // the pixel under Steve (band colours or a log)
                let inv = 255 - a; // the background's share
                let or = (((((src >> 16) & 0xFF) * a) + (((dst >> 16) & 0xFF) * inv) + 127) / 255) & 0xFF; // red over
                let og = (((((src >> 8) & 0xFF) * a) + (((dst >> 8) & 0xFF) * inv) + 127) / 255) & 0xFF; // green over
                let ob = (((((src) & 0xFF) * a) + (((dst) & 0xFF) * inv) + 127) / 255) & 0xFF; // blue over
                *dib_at(tx, ty) = 0xFF00_0000 | (or << 16) | (og << 8) | ob; // write the blend
            }
            tx += 1;
        }
        ty += 1;
    }
}

// Java's image(playerHitbox, x, y) for the debug mode: draw the red overlay
// for every visible pixel of a mask, clipped to the current dirty rect.
// The red is blended like Java's translucent hitbox colour.
unsafe fn blit_mask_avatar(mask: &[u64], row_bits: usize, w: i32, h: i32, x: i32, y: i32, r: Rect) {
    let x0 = x.max(r.x0); // clip to the dirty rect
    let x1 = (x + w).min(r.x1);
    let y0 = y.max(r.y0);
    let y1 = (y + h).min(r.y1);
    let mut ty = y0; // first overlay row
    while ty < y1 { // row by row
        let mut tx = x0; // first overlay column
        while tx < x1 { // pixel by pixel
            let idx = (ty - y) as usize * row_bits + (tx - x) as usize; // the mask bit for this pixel
            if mask[idx / 64] >> (idx % 64) & 1 != 0 { // Java: getAlpha(...) >= minVisibleAlpha
                let dst = *dib_at(tx, ty); // what's under the overlay
                let a = HITBOX_COLOR >> 24; // the red overlay's alpha (0x78)
                let inv = 255 - a; // the shared transparency maths
                let or = (((((HITBOX_COLOR >> 16) & 0xFF) * a) + (((dst >> 16) & 0xFF) * inv) + 127) / 255) & 0xFF; // blend red
                let og = (((((HITBOX_COLOR >> 8) & 0xFF) * a) + (((dst >> 8) & 0xFF) * inv) + 127) / 255) & 0xFF; // blend green
                let ob = (((((HITBOX_COLOR) & 0xFF) * a) + (((dst) & 0xFF) * inv) + 127) / 255) & 0xFF; // blend blue
                *dib_at(tx, ty) = 0xFF00_0000 | (or << 16) | (og << 8) | ob; // write the tinted pixel
            }
            tx += 1;
        }
        ty += 1;
    }
}

// Java's rect(x, y, w, h) with a stroke and no fill: a 1px outline (drawn on
// the box's edges), clipped to the current dirty rect.
unsafe fn outline_rect(x: i32, y: i32, w: i32, h: i32, r: Rect) {
    let bx = x.max(r.x0); // clip borders to the dirty rect
    let bx1 = (x + w).min(r.x1);
    // top and bottom rows
    if y >= r.y0 && y < r.y1 { // the top edge falls inside the rect
        fill_span(bx, bx1, y, OUTLINE_COLOR); // Java's upper edge
    }
    if y + h - 1 >= r.y0 && y + h - 1 < r.y1 { // the bottom edge
        fill_span(bx, bx1, y + h - 1, OUTLINE_COLOR);
    }
    // left and right columns
    if x >= r.x0 && x < r.x1 { // the left edge
        let mut ty = y.max(r.y0); // clipped top
        let end = (y + h).min(r.y1); // clipped bottom
        while ty < end {
            *dib_at(x, ty) = OUTLINE_COLOR; // one yellow pixel
            ty += 1;
        }
    }
    if x + w - 1 >= r.x0 && x + w - 1 < r.x1 { // the right edge
        let mut ty = y.max(r.y0);
        let end = (y + h).min(r.y1);
        while ty < end {
            *dib_at(x + w - 1, ty) = OUTLINE_COLOR;
            ty += 1;
        }
    }
}

// Java's drawGame(), but only inside one dirty rect (the whole frame stays
// deterministic because every rect composes bands + sprites + overlays).
unsafe fn compose_rect(g: &Game, r: Rect) {
    let r = r.clamp_client(); // never paint outside the client area
    if r.is_empty() { // nothing to do for empty rects
        return;
    }
    fill_rect(r, BG_COLOR); // Java: background(backgroundColor)
    // Java's drawRoads(): two road bands behind everything
    for road_y in [ROAD_ONE_Y, ROAD_TWO_Y] { // both roads
        let top = road_y.max(r.y0); // the band's visible top
        let bottom = (road_y + ROAD_HEIGHT).min(r.y1); // the band's visible bottom
        for y in top..bottom { // only the intersection rows
            fill_span(r.x0, r.x1, y, ROAD_COLOR); // Java: fill(roadColor) rect(...)
        }
    }
    // Java's drawRiver(): one water band behind everything
    {
        let top = RIVER_Y.max(r.y0); // the river's visible top
        let bottom = (RIVER_Y + RIVER_HEIGHT).min(r.y1); // ... and bottom
        for y in top..bottom { // only the intersection rows
            fill_span(r.x0, r.x1, y, RIVER_COLOR); // Java: fill(riverColor) rect(...)
        }
    }
    // Java's drawLogs(), then drawVehicles(), then drawPlayer() (that order)
    for i in 0..MAX_LOGS { // every still-floating log
        let l = g.logs[i]; // the slot's data
        if l.active { // Java: if (log != null)
            let pixels = LOG_SLOTS.get()[i].as_ptr(); // this slot's resized pixels
            blit_flat(pixels, LOG_SLOT_W, l.w, l.h, l.x, l.y, r); // opaque blit
        }
    }
    for i in 0..MAX_VEHICLES { // every live vehicle
        let v = g.vehicles[i];
        if v.active { // Java: if (vehicle != null)
            let vi = (v.kind * 2 + v.dir) as usize; // Java: which vehicle image this is
            blit_flat(VEHICLE_DATA.get()[vi * VEH_PX..].as_ptr(), VEHICLE_W, VEHICLE_W, VEHICLE_H, v.x, v.y, r); // flattened sprite
        }
    }
    blit_player(r); // Java's drawPlayer(): alpha blend, scaled by the jump
    // Java's drawHitboxes(): only while debug mode is on
    if g.debug_mode { // Java: if (debugMode)
        for i in 0..MAX_VEHICLES {
            let v = g.vehicles[i];
            if v.active { // vehicles: yellow box + red pixel overlay
                let vi = (v.kind * 2 + v.dir) as usize; // which sprite/mask
                let words = mask_words(VEHICLE_W, VEHICLE_H); // mask stride
                outline_rect(v.x, v.y, VEHICLE_W, VEHICLE_H, r); // Java: rect(vehicle.x, ...)
                blit_mask_avatar(&VEHICLE_MASK.get()[vi * words..(vi + 1) * words], VEHICLE_W as usize, VEHICLE_W, VEHICLE_H, v.x, v.y, r); // Java: image(vehicle.hitbox, ...)
            }
        }
        for i in 0..MAX_LOGS { // logs: yellow box only (the safe-centre region)
            let l = g.logs[i];
            if l.active {
                outline_rect(l.x, l.y, l.w, l.h, r); // Java: rect(log.x, ...)
            }
        }
        outline_rect(g.player_x, g.player_y, PLAYER_W, PLAYER_H, r); // Java: rect(playerX, ...) â€” Steve's base box
        blit_mask_avatar(PLAYER_MASK.get(), PLAYER_W as usize, PLAYER_W, PLAYER_H, g.player_x, g.player_y, r); // Java: image(playerHitbox, ...)
    }
}

// Merge rectangles (Java repaints everything; we BitBlt the fewest rects).
// Two rects that touch or overlap are united; rectangles that never touch
// stay separate. O(nÂ²) over at most a few dozen rects is nothing per frame.
fn drain_dirty() -> ([Rect; 64], usize) { // merged rects + how many are valid
    let mut count = *DIRTY_N.get(); // what's pending
    let src = DIRTY.get(); // the game's live list
    let mut rects = [Rect::EMPTY; 64]; // a drained copy onto this frame's stack
    let mut i = 0usize; // cursor for the copy
    while i < count { // copy the pending set out
        rects[i] = src[i]; // one rect at a time
        i += 1;
    }
    *DIRTY_N.get() = 0; // the game starts filling the same list again, fresh
    let mut changed = true; // repeat until no merges happen
    while changed {
        changed = false; // assume a clean pass
        let mut a = 0usize; // first of the pair
        while a < count { // walk the list
            let mut b = a + 1; // second of the pair
            while b < count { // against every later rect
                if rects[a].x0 <= rects[b].x1 && rects[b].x0 <= rects[a].x1 // touching/overlapping horizontally
                    && rects[a].y0 <= rects[b].y1 && rects[b].y0 <= rects[a].y1 { // and vertically
                    rects[a] = rects[a].union_with(rects[b]); // grow the first
                    count -= 1; // the second is absorbed
                    rects[b] = rects[count]; // and the list tail fills its hole
                    changed = true; // one merge means a new pass might merge more
                } else {
                    b += 1; // keep comparing
                }
            }
            a += 1;
        }
    }
    (rects, count) // in arbitrary order (the caller clips each one anyway)
}

// -- the FPS counter and the window flush --------------------------------

static FPS_FRAMES: Global<u32> = Global(UnsafeCell::new(0)); // frames since the last measured window
static FPS_WINDOW_START: Global<i64> = Global(UnsafeCell::new(0)); // when that window began
static FPS_SHOWN: Global<i32> = Global(UnsafeCell::new(0)); // the count currently on the screen
static FPS_TEXT: Global<([u16; 24], usize)> = Global(UnsafeCell::new(([0; 24], 0))); // current "FPS: n/60" as UTF-16

// Java: "FPS: " + round(frameRate) + "/" + framesPerSecond, rebuilt once a
// second with this build's own measured count (Java reads its own counter).
fn refresh_fps_text() {
    let g = fps_parts(); // snapshot the counters
    let text = g.0; // the fresh "FPS: n/60" line
    let len = g.1; // its UTF-16 length
    FPS_TEXT.get().0 = text; // store the glyphs
    FPS_TEXT.get().1 = len; // and the length
}

// Build the text line for a freshly measured frames-per-second count.
fn fps_parts() -> ([u16; 24], usize) {
    let mut buf: [u16; 24] = [0; 24]; // "FPS: " (5) + digits + "/60" (3) + NUL
    const PREFIX: &[u16] = &[0x0046, 0x0050, 0x0053, 0x003A, 0x0020]; // 'F','P','S',':',' '
    const SUFFIX: &[u16] = &[0x002F, 0x0036, 0x0030]; // '/','6','0'
    let mut n = 0usize; // write cursor
    for &c in PREFIX { // "FPS: "
        buf[n] = c;
        n += 1;
    }
    let mut digits: [u16; 4] = [0; 4]; // room for any realistic count (Java caps nothing)
    let mut m = 4usize; // digits write right to left
    let mut c = (*FPS_SHOWN.get()).max(1) as u32; // Java's round(frameRate), floored at 1 like its first paint
    while c > 0 { // collect each decimal digit
        m -= 1;
        digits[m] = b'0' as u16 + (c % 10) as u16; // one digit
        c /= 10; // drop it
    }
    while m < 4 { // most significant first
        buf[n] = digits[m]; // no zero padding: Java prints plain digits
        n += 1;
        m += 1;
    }
    for &c in SUFFIX { // "/60"
        buf[n] = c;
        n += 1;
    }
    (buf, n) // the assembled line
}

// Java's drawFps(): fill the counter's box, then TextOutW in GDI (the only
// non-software draw in the build), always as the last step of every frame.
unsafe fn draw_fps() {
    let text = FPS_TEXT.get(); // the UTF-16 line
    let r = FPS_RECT; // Java's fpsMargin box
    fill_rect(r, BG_COLOR); // black behind the glyphs (Java's text floats on the redraw)
    TextOutW(*BACK_DC.get(), FPS_MARGIN, FPS_MARGIN, text.0.as_ptr(), text.1 as i32); // Java: text("FPS: n/60", 10, 10)
}

// The frame's last step: paint every dirty rect (Java: the whole window) and
// BitBlt each one to the window, then the FPS text as its own little rect.
unsafe fn draw_frame(window: H) {
    let window_dc = GetDC(window); // the screen's dc for this frame only
    let (rects, count) = drain_dirty(); // the merged list of changed regions
    let mut i = 0usize; // cursor
    while i < count { // compose + blit each
        let g = GAME.get(); // the game state (shared-borrow checker: one get per rect)
        let r = rects[i]; // this rect
        compose_rect(g, r); // bands, sprites, overlays â€” Java's frame, cut to r
        let w = r.x1 - r.x0; // rect width
        let h = r.y1 - r.y0; // rect height
        BitBlt(window_dc, r.x0, r.y0, w, h, *BACK_DC.get(), r.x0, r.y0, SRCCOPY); // the flip: buffer -> screen
        i += 1;
    }
    draw_fps(); // Java's drawFps() â€” always last in the draw order
    BitBlt(window_dc, FPS_RECT.x0, FPS_RECT.y0, FPS_TEXT_W, FPS_TEXT_H, *BACK_DC.get(), FPS_RECT.x0, FPS_RECT.y0, SRCCOPY); // the counter's own flip
    ReleaseDC(window, window_dc); // give the window dc back
    *FPS_FRAMES.get() += 1; // one more frame measured
    let now = millis(); // the current time
    if *FPS_WINDOW_START.get() == 0 { // first frame ever
        *FPS_WINDOW_START.get() = now; // start the first measurement window
    } else if now - *FPS_WINDOW_START.get() >= 1000 { // a full second passed
        *FPS_SHOWN.get() = (*FPS_FRAMES.get() as i32).clamp(1, 999); // measured count, mins 1 like Java's early frames
        *FPS_FRAMES.get() = 0; // restart the measurement
        *FPS_WINDOW_START.get() = now; // next window begins now
        refresh_fps_text(); // Java regenerates its string every frame
    }
}

// ---- the window, the keys, and the frame loop --------------------------

// Java's keyPressed(keyCode): a first press steps at once, Shift+D is debug.
unsafe fn key_down(g: &mut Game, vk: usize) {
    match vk {
        VK_SPACE => { // Java: if (key == ' ' && !spaceBarDown)
            if !g.space_bar_down { // only the first press jumps (auto-repeat is filtered in Java too)
                g.space_bar_down = true; // Java: spaceBarDown = true
                jump_player(g); // Java: jumpPlayer()
            }
        }
        0x45 => { // 'E', the up key: Java: keyCode == 'E' || keyCode == UP
            if !g.moving_up { // Java: if (isPressed && !movingUp) stepPlayer(0, -playerSpeed)
                step_player(g, 0, -PLAYER_SPEED); // one step right away
            }
            g.moving_up = true; // Java: movingUp = isPressed
        }
        0x53 => { // 'S', the left key: Java: keyCode == 'S' || keyCode == LEFT
            if !g.moving_left {
                step_player(g, -PLAYER_SPEED, 0);
            }
            g.moving_left = true;
        }
        0x46 => { // 'F', the right key: Java: keyCode == 'F' || keyCode == RIGHT
            if !g.moving_right {
                step_player(g, PLAYER_SPEED, 0);
            }
            g.moving_right = true;
        }
        0x44 => { // 'D': Java checks `key == 'D'` BEFORE the direction keys â€”
            let shift = GetKeyState(VK_SHIFT as i32) as u16 & 0x8000 != 0; // shift held makes the 'D' key uppercase (the high bit of i16)
            let caps = GetKeyState(VK_CAPITAL as i32) as u16 & 1 != 0; // caps lock does in Java too
            if shift || caps { // Java: if (DEBUG_MODE && key == 'D')
                if DEBUG_MODE { // Java: key == 'D' (uppercase) toggles debug
                    g.debug_mode = !g.debug_mode; // Java: debugMode = !debugMode
                }
                // Java returns here: shift+D never moves Steve down
            } else { // plain 'd' walks down: Java: keyCode == 'D' â†’ movingDown
                if !g.moving_down {
                    step_player(g, 0, PLAYER_SPEED);
                }
                g.moving_down = true;
            }
        }
        VK_UP => { // Java: keyCode == UP â€” the same movingUp as 'E'
            if !g.moving_up {
                step_player(g, 0, -PLAYER_SPEED);
            }
            g.moving_up = true;
        }
        VK_LEFT => { // Java: keyCode == LEFT
            if !g.moving_left {
                step_player(g, -PLAYER_SPEED, 0);
            }
            g.moving_left = true;
        }
        VK_RIGHT => { // Java: keyCode == RIGHT
            if !g.moving_right {
                step_player(g, PLAYER_SPEED, 0);
            }
            g.moving_right = true;
        }
        VK_DOWN => { // Java: keyCode == DOWN
            if !g.moving_down {
                step_player(g, 0, PLAYER_SPEED);
            }
            g.moving_down = true;
        }
        _ => {} // any other key: Java's setDirection ignores codes it doesn't match
    }
}

// Java's keyReleased(): only this key's own direction clears (holding both
// 'S' and LEFT and releasing one stops the other in Java â€” we mirror it).
unsafe fn key_up(g: &mut Game, vk: usize) {
    match vk {
        VK_SPACE => g.space_bar_down = false, // Java: if (key == ' ') spaceBarDown = false
        0x45 | VK_UP => g.moving_up = false, // Java: movingUp = isPressed(false) for that key
        0x53 | VK_LEFT => g.moving_left = false,
        0x46 | VK_RIGHT => g.moving_right = false,
        0x44 | VK_DOWN => g.moving_down = false, // (Shift+D's keyup also clears movingDown; matches Java)
        _ => {} // other keys release nothing
    }
}

// Win32 calls this for every window event (same shape as pong's handler).
unsafe extern "system" fn window_proc(window: H, message: u32, wparam: usize, lparam: isize) -> isize {
    unsafe {
        match message {
            WM_ERASEBKGND => 1, // we paint every pixel ourselves: skip GDI's flicker fill
            WM_PAINT => { // an exposed window area must be repainted (exposure can outrun our frame rects)
                let mut ps: PaintStruct = core::mem::zeroed(); // paint info, zeroed as Win32 requires
                BeginPaint(window, &mut ps); // validates the area and gives us its rect
                push_dirty(ps.paint); // that area joins the next frame's compose list
                EndPaint(window, &ps); // done: Windows stops asking until something changes
                0
            }
            WM_KEYDOWN => { // a key went down (auto-repeat included, like Java's repeated keyPressed)
                key_down(GAME.get(), wparam); // Java's keyPressed() logic
                0
            }
            WM_KEYUP => { // a key came up
                key_up(GAME.get(), wparam); // Java's keyReleased() logic
                0
            }
            WM_KILLFOCUS => { // alt-tabbed away: Java has no cursor here, so don't drift Steve
                let g = GAME.get(); // the game state
                g.moving_up = false; // forget every held key,
                g.moving_left = false; // including the space bar's jump latch,
                g.moving_down = false; // otherwise Steve walks forever off screen
                g.moving_right = false; // (pong's build has this too)
                g.space_bar_down = false;
                0
            }
            WM_DESTROY => { // the window is closing
                PostQuitMessage(0); // end the main loop
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam), // everything else: Windows' default
        }
    }
}

// Create the window, the back buffer and the font, then pump frames at 60
// FPS with a high-resolution waitable timer, exactly like pong's loop and
// (in spirit) Processing's frameRate(60).
unsafe fn run() {
    unsafe {
        match load_images() { // Java loads its images on a thread during setup()
            Ok(()) => {} // the sprites are all ready
            Err(()) => { return; } // a missing or broken asset: give up silently
        }
        let g = GAME.get(); // the one game state
        *g = Game::new(); // Java's field initialisers
        place_player_at_start(g); // Java: placePlayerAtStart() after placeLanes()
        g.next_road_one = make_vehicle(ROAD_ONE_Y, true); // Java: nextRoadOneVehicle = makeVehicle(roadOneY, true)
        g.next_road_two = make_vehicle(ROAD_TWO_Y, false); // Java: nextRoadTwoVehicle = makeVehicle(roadTwoY, false)
        g.next_road_one_spawn = millis(); // Java: nextRoadOneSpawn = millis()
        g.next_road_two_spawn = millis() + ROAD_TWO_SPAWN_OFFSET; // Java: millis() + roadTwoSpawnOffset
        g.next_log_spawn = millis(); // Java: nextLogSpawn = millis()
        g.next_log_speed = rng_i32(MIN_LOG_SPEED, MAX_LOG_SPEED + 1); // Java: (int) random(minLogSpeed, maxLogSpeed + 1)
        *FPS_SHOWN.get() = 60; // Java's frameRate reads 60 (the target) on its first frames
        refresh_fps_text(); // and its string exists from frame one
        *FPS_WINDOW_START.get() = millis();
        push_dirty(Rect::FULL); // the very first paint covers the whole window

        // window class + window (the Java surface 1900x1000 P2D canvas)
        let instance = GetModuleHandleW(null()); // this exe's module handle
        let class_name = wz!("FroggerRsWindow"); // our window class's name
        let class = WndClass { // the registration record
            style: 0, // no class-wide style
            wnd_proc: window_proc, // our event callback
            cls_extra: 0, // no extra class bytes
            wnd_extra: 0, // no extra window bytes
            instance, // who owns the class
            icon: null_mut(), // default icon
            cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16), // standard arrow cursor
            background: GetStockObject(NULL_BRUSH), // we erase pixels ourselves (WM_ERASEBKGND answers 1)
            menu_name: null(), // no menu
            class_name, // the name defined above
        };
        if RegisterClassW(&class) == 0 { // 0 = failed to register
            return; // can't open a window: give up silently
        }
        let mut outer = Rect { x0: 0, y0: 0, x1: WINDOW_W, y1: WINDOW_H }; // the client's size...
        AdjustWindowRectEx(&mut outer, WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX, 0, 0); // ...expanded to the window's frame size
        let outer_w = outer.x1 - outer.x0; // full window width including borders
        let outer_h = outer.y1 - outer.y0; // full window height including the title bar
        let window = CreateWindowExW(0, class_name, wz!("Frogger DA Mogger"), WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX, // the Java game's title
                                     CW_USEDEFAULT, CW_USEDEFAULT, outer_w, outer_h, // default position, exact client size
                                     null_mut(), null_mut(), instance, null_mut()); // no parent, menu, or params
        if window.is_null() { // window creation failed
            return;
        }
        // the DIB back buffer: 1900x1000 top-down 32bpp, plus a memory dc to blit from
        let wdc = GetDC(window); // the window's screen dc (for buffer setup only)
        let back_dc = CreateCompatibleDC(wdc); // the memory dc that will own the pixels
        ReleaseDC(window, wdc); // the memory side doesn't need the screen dc afterwards
        let header = BmiHeader { // the bitmap info for a top-down DIB
            size: core::mem::size_of::<BmiHeader>() as u32, // struct's own size
            width: WINDOW_W, // pixels wide
            height: -WINDOW_H, // negative: row 0 is the TOP row, like the screen
            planes: 1, // always 1
            bit_count: 32, // 32bpp BGRA
            compression: BI_RGB, // raw pixels
            size_image: 0, // let Windows compute it
            x_pels: 0, y_pels: 0, clr_used: 0, clr_important: 0, // unused palette fields
        };
        let bmi = Bmi { header, colors: [0] }; // full BITMAPINFO (header + empty colour table)
        let mut bits: *mut u32 = null_mut(); // CreateDIBSection hands its pixels back here
        let bmp = CreateDIBSection(back_dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0); // allocate the buffer
        if bmp.is_null() || bits.is_null() { // allocation failed
            return;
        }
        SelectObject(back_dc, bmp); // the buffer's pixels are now reachable via back_dc
        *DIB.get() = bits; // our renderer writes straight into them
        *BACK_DC.get() = back_dc; // remembered for the BitBlts
        // the FPS counter's font (Java's createFont("Arial Bold", fpsTextSize))
        let font = CreateFontW(-FPS_SIZE, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, CLEARTYPE_QUALITY, 0, wz!("Arial Bold")); // negative height = em size
        SelectObject(back_dc, font); // keep it selected for every TextOutW
        SetTextColor(back_dc, 0x0000FF00); // Java: fpsColor = 0xFF00FF00 as 0xAARRGGBB; a COLORREF is 0x00BBGGRR, so green is 0x0000FF00
        SetBkMode(back_dc, TRANSPARENT); // no opaque box behind the glyphs
        SetTextAlign(back_dc, TA_LEFT | TA_TOP); // x,y = the text's top-left, like Java's textAlign(LEFT, TOP)
        *GAME_FONT.get() = font;
        ShowWindow(window, SW_SHOW); // show and activate (Java: surface visible)
        UpdateWindow(window); // paint now (WM_PAINT lands in our handler: full-window dirty push)

        // the frame pacer: a high-resolution waitable timer plus QPC, like pong
        let timer = CreateWaitableTimerExW(null(), null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS); // sub-ms timer (may fail: fall back)
        let (mut freq, mut now) = (0i64, 0i64); // clock frequency and current reading
        QueryPerformanceFrequency(&mut freq); // ticks per second
        let frame = freq / FPS as i64; // ticks per frame
        QueryPerformanceCounter(&mut now);
        let mut next_frame = now; // when the next frame is due
        let mut msg: Msg = core::mem::zeroed(); // message pump scratch space
        'running: loop { // the frame loop
            QueryPerformanceCounter(&mut now);
            if now < next_frame { // early for this frame: sleep the difference
                let wait_100ns = (next_frame - now) * 10_000_000 / freq; // ticks -> 100-nanosecond units
                if !timer.is_null() { // high-resolution timer available
                    let due = -wait_100ns; // negative = relative
                    SetWaitableTimer(timer, &due, 0, null(), null(), 0); // fire once after the wait
                    MsgWaitForMultipleObjects(1, &timer, 0, INFINITE, QS_ALLINPUT); // sleep until the timer or a message
                } else {
                    MsgWaitForMultipleObjects(0, null(), 0, (wait_100ns / 10_000).max(1) as u32, QS_ALLINPUT); // no timer: coarse millisecond wait
                }
            }
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 { // drain every pending message
                if msg.message == 0x0012 { // WM_QUIT
                    break 'running; // leave the frame loop
                }
                DispatchMessageW(&msg); // otherwise run it through window_proc
            }
            QueryPerformanceCounter(&mut now);
            if now >= next_frame { // time for the next frame?
                let g2 = GAME.get(); // the game state
                update_game(g2); // Java: updateGame() â€” moves, spawns, collisions
                draw_frame(window); // Java: drawGame() â€” dirty rects onto the screen
                next_frame += frame; // schedule the next step
                if next_frame <= now { // we're badly behind
                    next_frame = now + frame; // don't burst to catch up
                }
            }
        }
        // ExitProcess frees the font, bitmap, dc and timer handles.
    }
}

// Entry point when there is no C runtime at all: we ARE the CRT. build.rs
// adds -e,mainCRTStartup for the gnu-flavoured (gnullvm) link used here.
#[cfg(all(not(test), windows))]
#[unsafe(no_mangle)] // exported under the exact name the linker looks for
extern "system" fn mainCRTStartup() -> ! { // the exe's first function
    unsafe {
        run(); // play until quit (window closed)
        ExitProcess(0) // exit with code 0
    }
}

#[cfg(not(test))]
#[panic_handler] // no std, so we must define what a panic does
fn panic(_: &core::panic::PanicInfo) -> ! { // any panic: exit with code 1
    unsafe { ExitProcess(1) }
}

// The linker wants this marker whenever floating point is used without a C
// runtime (the jump's diagonal round and the tests touch floats).
#[cfg(not(test))]
#[unsafe(no_mangle)] // exported under the exact name the linker checks for
static _fltused: i32 = 0; // a dummy: the Microsoft linker just needs the symbol to exist

// ---- tests: `cargo test --release` compiles this file as a lib with std ----

#[cfg(test)]
mod tests {
    use super::*; // see everything above (the game has no hidden deps)

    // PNG z-axis decoding: zlib-decompressing a stored block round-trips.
    #[test]
    fn inflate_returns_a_stored_block() {
        let mut data = vec![0x78u8, 0x01]; // zlib header CMF/FLG (passes the mod-31 check)
        data.extend_from_slice(&[0x01, 0x07, 0x00, 0xF8, 0xFF]); // final stored block: BFINAL+BTYPE, LEN=7, NLEN=0xFFF8
        data.extend_from_slice(b"frogger"); // the block's seven literal bytes
        let mut out = [0u8; 7]; // the exact expected size (the caller always says how much)
        inflate(&data, &mut out, 7).expect("stored block inflates");
        assert_eq!(&out, b"frogger"); // plain bytes, unchanged
        assert!(inflate(&data, &mut out, 5).is_err()); // wrong expected size -> Err
    }

    // PNG's five filters, reversed by unfilter() on a made-up picture.
    #[test]
    fn unfilter_rebuilds_every_filter_type() {
        let (w, h) = (6usize, 5usize); // small enough to eyeball, every filter used
        let row: usize = w * 4; // image bytes per row
        let stride: usize = row + 1; // + the filter byte
        let mut recon = Vec::with_capacity(row * h); // the true RGBA bytes per row
        for i in 0..row * h { // a deterministic spread over all byte values
            recon.push((i * 31 + 11) as u8);
        }
        let mut zs = vec![0u8; stride * h]; // what the PNG stores
        for r in 0..h { // encode row r with filter r % 5
            let f = r % 5; // all five filters appear
            zs[r * stride] = f as u8; // the filter byte
            for x in 0..row { // one filtered byte per image byte
                let i = r * row + x; // this byte's index in recon
                let at = |rr: usize, xx: usize| recon[rr * row + xx]; // helper reads
                let cur = recon[i]; // the true byte
                let a = if x >= 4 { at(r, x - 4) } else { 0 }; // Sub/Average/Paeth neighbours
                let b = if r > 0 { at(r - 1, x) } else { 0 }; // Up neighbour
                let c = if x >= 4 && r > 0 { at(r - 1, x - 4) } else { 0 }; // Upper-Left neighbour
                let pred: u8 = match f {
                    0 => 0, // None predicts zero
                    1 => a, // Sub predicts the left byte
                    2 => b, // Up predicts the above byte
                    3 => ((a as u32 + b as u32) >> 1) as u8, // Average predicts the mean
                    _ => { // Paeth, RFC 1951's gradient trick
                        let p = a as i32 + b as i32 - c as i32;
                        let (pa, pb, pc) = ((p - a as i32).abs(), (p - b as i32).abs(), (p - c as i32).abs());
                        if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c }
                    }
                };
                zs[r * stride + 1 + x] = cur.wrapping_sub(pred); // delta = recon - pred (mod 256)
            }
        }
        unfilter(&mut zs, w, h); // reverse it in place
        let mut same = true; // everything must come back
        for r in 0..h {
            for x in 0..row {
                same &= zs[r * stride + 1 + x] == recon[r * row + x];
            }
        }
        assert!(same, "unfilter must reconstruct every byte of every filter");
    }

    // The real asset files decode to the System.Drawing reference pixels.
    #[test]
    fn decodes_real_assets_to_reference_pixels() {
        // steve.png (160x291)
        let mut f = std::fs::read("assets/images/steve/steve.png").expect("steve.png readable");
        let mut zs = vec![0u8; (160 * 4 + 1) * 291]; // inflate area sized exactly like the game would
        let (w, h) = decode_png(&mut f, &mut zs).expect("steve decodes");
        assert_eq!((w, h), (160, 291));
        let px = |x: usize, y: usize| -> u32 {
            let at = y * (w * 4 + 1) + 1 + x * 4;
            (zs[at + 3] as u32) << 24 | (zs[at + 2] as u32) << 16 | (zs[at + 1] as u32) << 8 | zs[at] as u32
        };
        assert_eq!(px(80, 145), 0xFFA6A703); // System.Drawing: R166 G167 B3 A255
        assert_eq!(px(0, 0) >> 24, 0); // corners are transparent
        assert_eq!(px(159, 290) >> 24, 0);

        // car_left.png (2172x724) â€” also covers the caBX ancillary chunk skip
        let mut f = std::fs::read("assets/images/car/car_left.png").expect("car readable");
        let mut zs = vec![0u8; ZSTRIDE_CAP]; // the vehicle stream is the biggest inflate
        let (w, h) = decode_png(&mut f, &mut zs).expect("car_left decodes");
        assert_eq!((w, h), (2172, 724));
        let px = |x: usize, y: usize| -> u32 {
            let at = y * (w * 4 + 1) + 1 + x * 4;
            (zs[at + 3] as u32) << 24 | (zs[at + 2] as u32) << 16 | (zs[at + 1] as u32) << 8 | zs[at] as u32
        };
        assert_eq!(px(1086, 362), 0xFDE4E5E5); // System.Drawing: R228 G229 B229 A253
        assert_eq!(px(0, 0) >> 24, 0); // transparent corner
        assert_eq!(px(2171, 723) >> 24, 0); // other transparent corner
        assert_eq!(px(500, 200) >> 24, 0); // transparent spot mid-sprite

        // birch_log.png (1167x445)
        let mut f = std::fs::read("assets/images/logs/birch_log.png").expect("birch readable");
        let mut zs = vec![0u8; (1167 * 4 + 1) * 445];
        let (w, h) = decode_png(&mut f, &mut zs).expect("birch decodes");
        assert_eq!((w, h), (1167, 445));
        let px = |x: usize, y: usize| -> u32 {
            let at = y * (w * 4 + 1) + 1 + x * 4;
            (zs[at + 3] as u32) << 24 | (zs[at + 2] as u32) << 16 | (zs[at + 1] as u32) << 8 | zs[at] as u32
        };
        assert_eq!(px(711, 222), 0xFDD3E2EF); // System.Drawing: R211 G226 B239 A253
        assert_eq!(px(600, 300), 0xFDC2D2E4); // R194 G210 B228 A253
        assert_eq!(px(0, 0) >> 24, 0); // transparent corner
    }

    // Java's resizeSharp picks exactly sx = x*srcW/dstW, sy = y*srcH/dstH.
    #[test]
    fn resize_sharp_matches_java_pixel_picks() {
        let mut f = std::fs::read("assets/images/steve/steve.png").expect("steve readable");
        let mut zs = vec![0u8; (160 * 4 + 1) * 291];
        let (w, h) = decode_png(&mut f, &mut zs).expect("steve decodes");
        let mut out = [0u32; PLAYER_PX]; // 74x135, the game's own Steve size
        resize_from_raw(&zs, w, h, &mut out, PLAYER_W as usize, PLAYER_H as usize, PLAYER_W as usize, None);
        for &(dx, dy) in &[(0, 0), (40, 67), (73, 134), (1, 1)] { // spot checks
            let sx = dx * w / PLAYER_W as usize; // Java: x * original.width / newWidth
            let sy = dy * h / PLAYER_H as usize; // Java: y * original.height / newHeight
            let at = sy * (w * 4 + 1) + 1 + sx * 4; // the source pixel in the decode
            let expected = (zs[at + 3] as u32) << 24 | (zs[at + 2] as u32) << 16 | (zs[at + 1] as u32) << 8 | zs[at] as u32;
            assert_eq!(out[dy * PLAYER_W as usize + dx], expected, "destination ({},{}) copies source ({},{})", dx, dy, sx, sy);
        }
    }

    // flatten(): Java's alpha-over-one-solid-colour blend.
    #[test]
    fn flatten_blends_like_java() {
        assert_eq!(flatten(0xFF10_2040, ROAD_COLOR), 0xFF10_2040); // opaque keeps its own colour
        assert_eq!(flatten(0x80FF_0000, BG_COLOR), 0xFF80_0000); // half red over black = 128 red
        assert_eq!(flatten(0x0000_0000, RIVER_COLOR), 0xFF1E_6BD6); // clear becomes the water
        // quarter red over black: (255*64 + 127)/255 = 64
        assert_eq!(flatten(0x40FF_0000, BG_COLOR), 0xFF40_0000);
    }

    // Java's hasRoomFor / hasRoomForLog gap predictions.
    #[test]
    fn spawn_gap_predictions_match_java() {
        let mut g = Game::new();
        // logs: no last log yet
        assert!(has_room_for_log(&g, 2));
        g.has_last_log = true;
        g.last_log = LogSlot { active: true, x: 200, y: 0, w: 100, h: 90, speed: 2, prev_x: 0 };
        assert!(has_room_for_log(&g, 2)); // same speed: 200 stays >= logGap
        assert!(!has_room_for_log(&g, 4)); // faster catches up: (4-2)*(1900-200)/2 = 1700 > gap
        g.last_log.x = 100;
        assert!(!has_room_for_log(&g, 2)); // still floating in: below logGap
        g.last_log.x = 1901;
        assert!(has_room_for_log(&g, 4)); // already past the right edge
        // vehicles: no last vehicle yet
        let new_fast = Vehicle { active: false, x: -VEHICLE_W, y: ROAD_ONE_Y + VEHICLE_PADDING, speed: 10, tallness: 40, kind: 0, dir: 1, prev_x: 0 };
        let new_slow = Vehicle { active: false, x: -VEHICLE_W, y: ROAD_ONE_Y + VEHICLE_PADDING, speed: 5, tallness: 40, kind: 0, dir: 1, prev_x: 0 };
        assert!(has_room_for(new_fast, None)); // Java: vehicleAhead == null -> true
        let ahead = Vehicle { active: true, x: 300, y: ROAD_ONE_Y + VEHICLE_PADDING, speed: 5, tallness: 40, kind: 0, dir: 1, prev_x: 0 };
        assert!(!has_room_for(new_fast, Some(ahead))); // (10-5)*(1900-300)/5 = 1600 gained -> too close
        assert!(has_room_for(new_slow, Some(ahead))); // (5-5)*320 = 0 -> 300 >= gap
        let near = Vehicle { active: true, x: 100, y: ROAD_ONE_Y + VEHICLE_PADDING, speed: 5, tallness: 40, kind: 0, dir: 1, prev_x: 0 };
        assert!(!has_room_for(new_slow, Some(near))); // within the gap itself
        let far = Vehicle { active: true, x: 1600, y: ROAD_ONE_Y + VEHICLE_PADDING, speed: 5, tallness: 40, kind: 0, dir: 1, prev_x: 0 };
        assert!(has_room_for(new_slow, Some(far))); // 1600 - 0 >= gap
        let leaving_left = Vehicle { active: true, x: -500, y: ROAD_TWO_Y + VEHICLE_PADDING, speed: -4, tallness: 100, kind: 1, dir: 0, prev_x: 0 };
        assert!(has_room_for(new_slow, Some(leaving_left))); // distance = width - (-500 + 420) = 1980 > width -> true
    }

    // Java's jump: 15 frames, exactly 240 px straight, an int-math arc of 59.
    #[test]
    fn jump_is_a_fifteen_frame_240_pixel_arc() {
        let mut g = Game::new();
        place_player_at_start(&mut g);
        g.moving_right = true;
        let x0 = g.player_x;
        jump_player(&mut g); // allowed: jumpStartTime = -cooldown
        let mut frames = 0; // how many update frames the jump lasts
        let mut max_h = 0; // the arc's top
        while g.jumping { // Java: updateJump() while isJumping
            update_jump(&mut g);
            frames += 1;
            max_h = max_h.max(g.jump_height_now);
        }
        assert_eq!(frames, JUMP_FRAMES); // 15 frames (250 ms at 60 fps)
        assert_eq!(g.player_x - x0, JUMP_LENGTH); // no pixels lost to rounding
        assert_eq!(g.jump_height_now, 0); // landed back on the ground
        assert_eq!(max_h, 4 * JUMP_HEIGHT * 7 * 8 / (JUMP_FRAMES * JUMP_FRAMES)); // 4*60*7*8/225 = 59, the int-math top of the arc
        // diagonal: every frame's step is round(16 / sqrt(2)) = 11, times 15 frames
        let mut g = Game::new();
        place_player_at_start(&mut g);
        g.player_x = 400; // away from every wall, so the diagonal never wraps (y0 300 + 165 stays inside)
        g.player_y = 300;
        g.moving_right = true;
        g.moving_down = true;
        let (x0, y0) = (g.player_x, g.player_y);
        jump_player(&mut g);
        while g.jumping {
            update_jump(&mut g);
        }
        assert_eq!(g.player_x - x0, 15 * round_half_up(JUMP_LENGTH as f64 / JUMP_FRAMES as f64 / SQRT_2) as i32); // 165 px
        assert_eq!(g.player_y - y0, g.player_x - x0); // both axes shrink together
        // the cooldown blocks an immediate re-jump (jump_start_time was just set)
        let was_jumping = g.jumping;
        jump_player(&mut g);
        assert_eq!(g.jumping, was_jumping); // still not jumping again
    }

    // placeLanes()'s vertical layout and the sprite resize maths.
    #[test]
    fn lanes_and_sprite_sizes_match_java() {
        assert_eq!(ROAD_ONE_Y, 140); // (1000 - (150*2 + 135*2 + 150)) / 2
        assert_eq!(RIVER_Y, 425); // 140 + 150 + 135
        assert_eq!(ROAD_TWO_Y, 710); // 425 + 150 + 135
        assert_eq!(VEHICLE_W, 420); // 2172 * 140 / 724
        assert_eq!(PLAYER_W, 74); // 160 * 135 / 291
        assert_eq!(MAX_LOG_H, 112); // 150 * 3 / 4
        assert_eq!(MIN_LOG_H, 84); // 112 * 3 / 4
        assert_eq!(JUMP_FRAMES, 15); // 250 * 60 / 1000
    }

    // The word-AND pixel test on synthetic masks, plus the AABB precheck.
    #[test]
    fn mask_and_game_collisions_match_java() {
        let mut steve_px = [0u32; PLAYER_PX]; // Steve: fully visible on his right half
        for y in 0..PLAYER_H as usize {
            for x in 0..PLAYER_W as usize {
                steve_px[y * PLAYER_W as usize + x] = if x >= PLAYER_W as usize / 2 { 0xC800_0000 } else { 0 }; // alpha 200 >= 128: visible
            }
        }
        let veh_left = |visible_left: bool| { // a vehicle visible on its left third or its right third
            let mut px = [0u32; VEH_PX];
            for y in 0..VEHICLE_H as usize {
                for x in 0..VEHICLE_W as usize {
                    let vis = if visible_left { x < VEHICLE_W as usize / 3 } else { x >= VEHICLE_W as usize * 2 / 3 }; // which third shows
                    px[y * VEHICLE_W as usize + x] = if vis { 0xFF00_0000 } else { 0 }; // alpha 255 where visible
                }
            }
            px
        };
        let build = |px: &[u32], w: usize, h: usize, words: usize| -> Vec<u64> {
            let mut m = vec![0u64; words];
            build_mask(px, w, h, w, &mut m);
            m
        };
        let pm = build(&steve_px, PLAYER_W as usize, PLAYER_H as usize, mask_words(PLAYER_W, PLAYER_H)); // Steve's bits
        let vl = build(&veh_left(true), VEHICLE_W as usize, VEHICLE_H as usize, mask_words(VEHICLE_W, VEHICLE_H)); // visible on the left third
        let vr = build(&veh_left(false), VEHICLE_W as usize, VEHICLE_H as usize, mask_words(VEHICLE_W, VEHICLE_H)); // visible on the right third
        // Steve's box: [913, 987) x [828, 963). A vehicle box at x=850 spans
        // [850, 1270) x [715, 855): they overlap on [913, 987) x [828, 855)
        // â€” cols 63..74 of the vehicle â€” inside its left third, so both show.
        let (x0, y0, x1, y1) = (913, 828, 987, 855); // the clipped overlap, exactly what is_player_hitting feeds in
        assert!(masks_touch(&pm, &vl, 913, 828, 850, 715, x0, y0, x1, y1)); // both sprites show in the window: hit
        assert!(!masks_touch(&pm, &vr, 913, 828, 850, 715, x0, y0, x1, y1)); // boxes touch but the vehicle's visible pixels lie elsewhere
        assert!(!masks_touch(&pm, &vl, 913, 828, 1200, 715, x0, y0, x1, y1)); // boxes disjoint: no shared pixel either
        let ghost = [0u32; VEH_PX]; // a fully transparent vehicle
        let gm = build(&ghost, VEHICLE_W as usize, VEHICLE_H as usize, mask_words(VEHICLE_W, VEHICLE_H));
        assert!(!masks_touch(&pm, &gm, 913, 828, 850, 715, x0, y0, x1, y1)); // nothing visible at all: Java's scan can't hit either
    }

    // Processing parity traps: round()'s half-up, RNG ranges, the FPS line.
    #[test]
    fn helpers_behave_like_processing() {
        assert_eq!(round_half_up(0.4), 0);
        assert_eq!(round_half_up(0.5), 1); // Java round() rounds .5 up
        assert_eq!(round_half_up(2.5), 3);
        assert_eq!(round_half_up(16.0 / SQRT_2), 11); // the diagonal step for 16
        assert_eq!(rng_i32(7, 7), 7); // one-value range
        for _ in 0..100 { // a spread of draws stays inside its bounds
            let k = rng_i32(1, 4); // Java's (int) random(1, 4) -> 1..3
            assert!(k >= 1 && k < 4);
            let h = rng_i32(MIN_LOG_H, MAX_LOG_H + 1); // the log height draw
            assert!(h >= MIN_LOG_H && h <= MAX_LOG_H);
        }
        *FPS_SHOWN.get() = 60; // simulate a measured 60
        let (text, len) = fps_parts();
        assert_eq!(&text[..len], &[0x0046, 0x0050, 0x0053, 0x003A, 0x0020, 0x0036, 0x0030, 0x002F, 0x0036, 0x0030]); // "FPS: 60/60"
        *FPS_SHOWN.get() = 123;
        let (_, len123) = fps_parts();
        assert_eq!(len123, 11); // "FPS: " (5) + "123" (3) + "/60" (3)
        *FPS_SHOWN.get() = 0;
    }

    // The dirty-rect merger: touching rects unite, disjoint ones stay split.
    #[test]
    fn dirty_rects_merge_when_touching() {
        *DIRTY_N.get() = 0;
        push_dirty(Rect::from_xywh(10, 10, 20, 20)); // [10..30)x[10..30)
        push_dirty(Rect::from_xywh(30, 10, 20, 20)); // touching on the right edge
        push_dirty(Rect::from_xywh(100, 100, 10, 10)); // far apart
        let (rects, n) = drain_dirty();
        assert_eq!(n, 2); // one merged + one separate
        assert_eq!(rects[0], Rect::from_xywh(10, 10, 40, 20)); // the union
        assert_eq!(rects[1], Rect::from_xywh(100, 100, 10, 10));
        push_dirty(Rect::from_xywh(-50, -50, 3000, 3000)); // one rect bigger than the window
        let (_, n) = drain_dirty();
        assert_eq!(n, 1); // clamped to the client, kept single
        *DIRTY_N.get() = 0; // leave the game list empty for the other tests
    }
}

