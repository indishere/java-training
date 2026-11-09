// No Rust standard library and no C runtime: the exe imports only from DLLs that ship with
// Windows (raw-dylib, so no import libraries or Visual Studio are needed to link it).
// `cargo test` builds with std and ordinary import libraries (no dlltool needed).
#![cfg_attr(not(test), no_std)] // no std library in the real exe (a test build keeps std)
#![cfg_attr(not(test), no_main)] // no auto-generated main: we hand-write the entry point below
#![windows_subsystem = "windows"] // GUI subsystem: launching opens no black console window
#![cfg_attr(test, allow(dead_code))] // tests don't call every helper, so silence that lint

use core::cell::UnsafeCell; // wraps shared mutable state so it can live in a `static`
use core::ffi::c_void; // C's void*, the type of every opaque Win32 handle
use core::ptr::{null, null_mut}; // const null pointer / mutable null pointer

const FPS: i32 = 60; // target frame rate: one game update every 1/60 s
const BALL_SIZE: i32 = 30; // the ball is a 30x30 square (the ellipse's diameter)
const PADDLE_WIDTH: i32 = 15; // paddle width in pixels
const PADDLE_HEIGHT: i32 = 100; // paddle height in pixels
const START_SPEED: f64 = 7.0; // ball speed: 7 pixels per frame at the start
const SPEED_UP: f64 = 1.05; // the ball gets 5% faster at every paddle hit
const AIM_SHIFT: f64 = 35.0; // how far off the paddle's middle to hit when aiming up or down

// ---- Win32: only the calls this game makes, declared by hand so no crates are needed ----

type H = *mut c_void; // generic Win32 handle (HWND, HDC, HFONT... are all opaque pointers)

#[repr(C)] // lay the fields out exactly like the C struct Win32 expects
#[derive(Clone, Copy)] // allow making copies (assignment, passing by value)
struct Rect {
    left: i32, // x of the left edge
    top: i32, // y of the top edge
    right: i32, // x of the right edge
    bottom: i32, // y of the bottom edge
}

#[repr(C)] // field layout must match C's WNDCLASSEX that RegisterClassW reads
struct WndClass {
    style: u32, // class style flags (0 = nothing special)
    wnd_proc: unsafe extern "system" fn(H, u32, usize, isize) -> isize, // pointer to our window callback
    cls_extra: i32, // extra bytes after the class struct (unused: 0)
    wnd_extra: i32, // extra bytes per window (unused: 0)
    instance: H, // handle to the exe's module
    icon: H, // window icon (null = default)
    cursor: H, // cursor shown over the window (the arrow)
    background: H, // brush used to paint the window background (black)
    menu_name: *const u16, // name of a class menu (null = none)
    class_name: *const u16, // the name we register this class under, UTF-16
}

#[repr(C)] // field layout must match C's MSG
struct Msg {
    hwnd: H, // window the message is for
    message: u32, // the message id (WM_PAINT, WM_KEYDOWN, ...)
    wparam: usize, // first message parameter (meaning depends on the message)
    lparam: isize, // second message parameter
    time: u32, // when the message was posted
    pt: [i32; 2], // cursor position at post time
    private: u32, // internal Windows field
}

#[repr(C)] // field layout must match C's PAINTSTRUCT
struct PaintStruct {
    hdc: H, // device context to paint into
    erase: i32, // whether the background needs erasing
    paint: Rect, // rectangle that needs repainting
    restore: i32, // reserved
    inc_update: i32, // reserved
    reserved: [u8; 32], // reserved padding
}

#[repr(C)] // field layout must match C's MONITORINFO
struct MonitorInfo {
    size: u32, // size of this struct, so Windows knows which version to fill
    monitor: Rect, // the whole monitor's rectangle
    work: Rect, // the monitor minus the taskbar
    flags: u32, // info flags (unused here)
}

#[cfg_attr(all(windows, not(test)), link(name = "user32", kind = "raw-dylib"))] // exe: import straight from user32.dll
#[cfg_attr(all(windows, test), link(name = "user32"))] // tests: link the normal import library
unsafe extern "system" {
    fn RegisterClassW(class: *const WndClass) -> u16; // register our window class, returns a class id
    fn CreateWindowExW(ex_style: u32, class: *const u16, title: *const u16, style: u32, // create a window of that class
                       x: i32, y: i32, w: i32, h: i32, // position and size
                       parent: H, menu: H, instance: H, param: H) -> H; // returns the window handle
    fn DefWindowProcW(hwnd: H, msg: u32, wparam: usize, lparam: isize) -> isize; // let Windows handle any message we ignore
    fn ShowWindow(hwnd: H, cmd: i32) -> i32; // show/maximize the window
    fn UpdateWindow(hwnd: H) -> i32; // force an immediate first paint
    fn PeekMessageW(msg: *mut Msg, hwnd: H, min: u32, max: u32, remove: u32) -> i32; // non-blocking message pump: take one message if any
    fn DispatchMessageW(msg: *const Msg) -> isize; // hand a message to window_proc
    fn PostMessageW(hwnd: H, msg: u32, wparam: usize, lparam: isize) -> i32; // queue a message for the window thread
    fn PostQuitMessage(code: i32); // queue WM_QUIT: makes the main loop exit
    fn MsgWaitForMultipleObjects(count: u32, handles: *const H, wait_all: i32, ms: u32, wake: u32) -> u32; // sleep until a timer fires or a message arrives
    fn InvalidateRect(hwnd: H, rect: *const Rect, erase: i32) -> i32; // mark a rectangle dirty so a WM_PAINT comes
    fn GetDC(hwnd: H) -> H; // borrow the screen's device context
    fn ReleaseDC(hwnd: H, dc: H) -> i32; // give the device context back
    fn BeginPaint(hwnd: H, ps: *mut PaintStruct) -> H; // start painting: fills ps, returns the paint dc
    fn EndPaint(hwnd: H, ps: *const PaintStruct) -> i32; // finish painting
    fn GetUpdateRgn(hwnd: H, rgn: H, erase: i32) -> i32; // fetch the dirty region as a region handle
    fn FillRect(dc: H, rect: *const Rect, brush: H) -> i32; // fill a rectangle with a brush
    fn LoadCursorW(instance: H, name: *const u16) -> H; // load a standard cursor by id
    fn GetWindowLongPtrW(hwnd: H, index: i32) -> isize; // read a window attribute (here: the style bits)
    fn SetWindowLongPtrW(hwnd: H, index: i32, value: isize) -> isize; // write a window attribute (the style bits)
    fn SetWindowPos(hwnd: H, after: H, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32; // move/resize the window
    fn MonitorFromWindow(hwnd: H, flags: u32) -> H; // which monitor the window is on
    fn GetMonitorInfoW(monitor: H, info: *mut MonitorInfo) -> i32; // fill in that monitor's rectangles
    fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex_style: u32) -> i32; // expand a client rect to the full window size (title bar, borders)
}

#[cfg_attr(all(windows, not(test)), link(name = "gdi32", kind = "raw-dylib"))] // exe: import straight from gdi32.dll
#[cfg_attr(all(windows, test), link(name = "gdi32"))] // tests: link the normal import library
unsafe extern "system" {
    fn GetStockObject(index: i32) -> H; // get a shared stock pen/brush/font (no cleanup needed)
    fn SetDCBrushColor(dc: H, color: u32) -> u32; // set the colour of the stock DC_BRUSH
    fn SetDCPenColor(dc: H, color: u32) -> u32; // set the colour of the stock DC_PEN
    fn SelectObject(dc: H, object: H) -> H; // pick an object into a dc, returns the previous one
    fn Ellipse(dc: H, left: i32, top: i32, right: i32, bottom: i32) -> i32; // draw an ellipse inside the box
    fn SetTextColor(dc: H, color: u32) -> u32; // text colour for TextOutW
    fn SetBkMode(dc: H, mode: i32) -> i32; // TRANSPARENT: don't draw a box behind the text
    fn SetTextAlign(dc: H, align: u32) -> u32; // how x,y are interpreted (centred, at baseline)
    fn TextOutW(dc: H, x: i32, y: i32, text: *const u16, len: i32) -> i32; // draw UTF-16 text at x,y
    fn CreateFontW(height: i32, width: i32, escapement: i32, orientation: i32, weight: i32, // create a font (negative height = character height
                   italic: u32, underline: u32, strike_out: u32, charset: u32, // decoration flags and character set
                   out_precision: u32, clip_precision: u32, quality: u32, // rendering hints
                   pitch_and_family: u32, face: *const u16) -> H; // family and font name
    fn DeleteObject(object: H) -> i32; // free a font/bitmap/region
    fn CreateCompatibleDC(dc: H) -> H; // off-screen dc for the back buffer
    fn CreateCompatibleBitmap(dc: H, w: i32, h: i32) -> H; // off-screen bitmap the right size
    fn DeleteDC(dc: H) -> i32; // free an off-screen dc
    fn BitBlt(dst: H, x: i32, y: i32, w: i32, h: i32, src: H, sx: i32, sy: i32, rop: u32) -> i32; // copy pixels src -> dst (the double-buffer flip)
    fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> H; // make an empty rectangular region
    fn SelectClipRgn(dc: H, rgn: H) -> i32; // limit drawing to a region (only redraw what's dirty)
}

#[cfg_attr(all(windows, not(test)), link(name = "kernel32", kind = "raw-dylib"))] // exe: import straight from kernel32.dll
#[cfg_attr(all(windows, test), link(name = "kernel32"))] // tests: link the normal import library
unsafe extern "system" {
    fn GetTickCount64() -> u64; // milliseconds since boot (used to randomise the first serve)
    fn GetModuleHandleW(name: *const u16) -> H; // this exe's module handle
    fn GetModuleFileNameW(module: H, name: *mut u16, size: u32) -> u32; // full path of the exe
    fn CreateWaitableTimerExW(attributes: *const c_void, name: *const u16, flags: u32, access: u32) -> H; // make a high-resolution waitable timer
    fn SetWaitableTimer(timer: H, due: *const i64, period: i32, // arm the timer; negative due = relative 100ns
                        completion: *const c_void, arg: *const c_void, resume: i32) -> i32; // no completion callback, no resume
    fn QueryPerformanceCounter(count: *mut i64) -> i32; // high-resolution clock reading
    fn QueryPerformanceFrequency(freq: *mut i64) -> i32; // how many clock ticks per second
    fn CreateFileW(name: *const u16, access: u32, share: u32, security: *const c_void, // open a file (here: .env)
                   disposition: u32, flags: u32, template: H) -> H; // open existing, no special flags
    fn ReadFile(file: H, buffer: *mut u8, len: u32, read: *mut u32, overlapped: *mut c_void) -> i32; // read bytes into a buffer
    fn CreateThread(security: *const c_void, stack: usize, start: unsafe extern "system" fn(H) -> u32, // start a worker thread on a function
                    param: H, flags: u32, id: *mut u32) -> H; // pass the parameter, don't create suspended
    fn GetLastError() -> u32; // error code of the last failing call
    fn CloseHandle(handle: H) -> i32; // close a file/thread/timer handle
    fn ExitProcess(code: u32) -> !; // kill the process now (also frees all handles)
}

#[cfg_attr(all(windows, not(test)), link(name = "winhttp", kind = "raw-dylib"))] // exe: import straight from winhttp.dll
#[cfg_attr(all(windows, test), link(name = "winhttp"))] // tests: link the normal import library
unsafe extern "system" {
    fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> H; // start an HTTP session (system proxy)
    fn WinHttpConnect(session: H, server: *const u16, port: u16, reserved: u32) -> H; // connect to a host:port
    fn WinHttpSetTimeouts(handle: H, resolve: i32, connect: i32, send: i32, receive: i32) -> i32; // 10 s timeouts for every stage
    fn WinHttpOpenRequest(connect: H, verb: *const u16, path: *const u16, version: *const u16, // build an HTTP request (POST, HTTPS)
                          referrer: *const u16, accept: *const *const u16, flags: u32) -> H; // no referrer, no accept types
    fn WinHttpSetOption(handle: H, option: u32, buffer: *const c_void, len: u32) -> i32; // tweak a handle option (cert revocation)
    fn WinHttpSendRequest(request: H, headers: *const u16, headers_len: u32, body: *const c_void, // send the request with headers and body
                          body_len: u32, total_len: u32, context: usize) -> i32; // headers_len -1 = null-terminated; context unused
    fn WinHttpReceiveResponse(request: H, reserved: *mut c_void) -> i32; // wait for the response headers
    fn WinHttpQueryHeaders(request: H, level: u32, name: *const u16, buffer: *mut c_void, // read a response header (the status code)
                           len: *mut u32, index: *mut u32) -> i32; // buffer and its size, no index
    fn WinHttpReadData(request: H, buffer: *mut c_void, len: u32, read: *mut u32) -> i32; // read a chunk of the response body
    fn WinHttpCloseHandle(handle: H) -> i32; // close a session/connection/request handle
}

// The compiler may emit calls to these for struct copies and zeroing (it depends on the
// opt-level). ntdll is mapped into every process anyway, and only what's used gets imported.
#[cfg(all(windows, not(test)))] // only for the real exe, not for tests
#[link(name = "ntdll", kind = "raw-dylib")] // ntdll.dll exports the C string/memory functions too
#[allow(dead_code)] // linking keeps unused ones anyway; rustc decides at link time
unsafe extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void; // copy n bytes
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void; // copy n bytes (overlap-safe)
    fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void; // fill n bytes with c
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32; // compare n bytes
}

const WM_DESTROY: u32 = 0x0002; // window is being destroyed
const WM_SIZE: u32 = 0x0005; // window was resized (new size in lparam)
const WM_KILLFOCUS: u32 = 0x0008; // window lost keyboard focus
const WM_PAINT: u32 = 0x000F; // something needs redrawing
const WM_QUIT: u32 = 0x0012; // posted by PostQuitMessage: exit the loop
const WM_ERASEBKGND: u32 = 0x0014; // background erase request (we decline: less flicker)
const WM_KEYDOWN: u32 = 0x0100; // a key was pressed
const WM_KEYUP: u32 = 0x0101; // a key was released
const WM_APP_SHOT: u32 = 0x8000; // WM_APP: a model answer, wparam = approach, lparam = shot
const VK_ESCAPE: usize = 0x1B; // Esc key virtual key code
const VK_UP: usize = 0x26; // Up arrow key
const VK_DOWN: usize = 0x28; // Down arrow key
const VK_F11: usize = 0x7A; // F11 key
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000; // a normal resizable window with title bar and borders
const WS_MAXIMIZE: u32 = 0x0100_0000; // style bit: window is maximized
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32; // "let Windows pick the position"
const SW_MAXIMIZE: i32 = 3; // ShowWindow command: show maximized
const PM_REMOVE: u32 = 1; // PeekMessageW: take the message out of the queue
const QS_ALLINPUT: u32 = 0x04FF; // wake on any input message
const INFINITE: u32 = 0xFFFF_FFFF; // wait forever (until the timer or a message)
const BLACK_BRUSH: i32 = 4; // stock object index: solid black brush
const DC_BRUSH: i32 = 18; // stock object index: colour-changeable brush
const DC_PEN: i32 = 19; // stock object index: colour-changeable pen
const TRANSPARENT: i32 = 1; // background mode: no opaque box behind text
const TA_CENTER: u32 = 6; // text alignment flag: centre horizontally
const TA_BASELINE: u32 = 24; // text alignment flag: y is the text baseline
const SRCCOPY: u32 = 0x00CC_0020; // BitBlt mode: plain pixel copy
const NULLREGION: i32 = 1; // GetUpdateRgn result: the dirty region is empty
const GWL_STYLE: i32 = -16; // window attribute index: the style bits
const SWP_NOZORDER: u32 = 0x0004; // SetWindowPos: don't change z-order
const SWP_FRAMECHANGED: u32 = 0x0020; // SetWindowPos: style changed, recompute the frame
const SWP_NOOWNERZORDER: u32 = 0x0200; // SetWindowPos: don't reorder the owner window
const MONITOR_DEFAULTTONEAREST: u32 = 2; // MonitorFromWindow: nearest monitor if not touching one
const IDC_ARROW: usize = 32512; // standard arrow cursor id
const FW_BOLD: i32 = 700; // font weight: bold
const DEFAULT_CHARSET: u32 = 1; // font charset: system default
const CLEARTYPE_QUALITY: u32 = 5; // font rendering: ClearType anti-aliasing
const CREATE_WAITABLE_TIMER_HIGH_RESOLUTION: u32 = 0x0000_0002; // timer flag: sub-millisecond accuracy
const TIMER_ALL_ACCESS: u32 = 0x001F_0003; // timer access mask: everything
const GENERIC_READ: u32 = 0x8000_0000; // file access: read only
const FILE_SHARE_READ: u32 = 1; // others may read the file while we have it open
const OPEN_EXISTING: u32 = 3; // file disposition: only open, never create
const INVALID_HANDLE: H = usize::MAX as H; // CreateFileW's failure return value (-1)
const STACK_SIZE_PARAM_IS_A_RESERVATION: u32 = 0x0001_0000; // CreateThread: stack size is a reservation, not committed
const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: u32 = 4; // system/PAC proxy, like the rest of Windows
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000; // request over HTTPS
const WINHTTP_OPTION_ENABLE_FEATURE: u32 = 79; // option id: enable a feature
const WINHTTP_ENABLE_SSL_REVOCATION: u32 = 1; // the feature: check certificate revocation
const WINHTTP_QUERY_STATUS_CODE: u32 = 19; // header selector: the status code
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000; // return the header as a number, not text
const ERROR_WINHTTP_SECURE_CERT_REV_FAILED: u32 = 12057; // error: couldn't check if the cert was revoked

const fn rgb(r: u32, g: u32, b: u32) -> u32 { // pack three 0-255 components into a COLORREF (0x00BBGGRR)
    r | g << 8 | b << 16 // red low byte, green middle, blue high
}

// ASCII string literal -> UTF-16 array, at compile time. `wz!` adds the terminating 0.
macro_rules! w { // "wide": &str -> &'static [u16] usable with the W Win32 functions
    ($s:expr) => {{
        const S: &str = $s; // the literal as a &str, in a const so it's compile-time
        const W: [u16; S.len()] = { // a u16 array of the same length, built at compile time
            let (b, mut out, mut i) = (S.as_bytes(), [0u16; S.len()], 0); // the ASCII bytes, the output, a counter
            while i < b.len() { // one byte at a time (const fns can't use iterators)
                out[i] = b[i] as u16; // ASCII widens to UTF-16 unchanged
                i += 1;
            }
            out // the finished array
        };
        &W // a reference to it, valid forever (statically allocated)
    }};
}
macro_rules! wz { // like w! but zero-terminated and returned as a raw pointer
    ($s:literal) => { w!(concat!($s, "\0")).as_ptr() }; // append the NUL, take the pointer
}

// Shared mutable state. The game is only touched on the window thread; the model buffers are
// handed to the one worker thread in flight and not touched again until it answers.
struct Global<T>(UnsafeCell<T>); // a Cell that Sync (shareable between threads) on purpose
unsafe impl<T> Sync for Global<T> {} // we promise access is actually co-ordinated (see above)
impl<T> Global<T> {
    #[allow(clippy::mut_from_ref)] // that's the whole point of this wrapper
    fn get(&self) -> &mut T { // hand out a &mut from a &self: the caller enforces the rules
        unsafe { &mut *self.0.get() } // dereference the raw interior pointer
    }
}

// Fixed-size append-only buffer; anything past the end is dropped (callers check `full`).
struct Buf<T: Copy, const N: usize> { // N elements max, element type must be plain-copyable
    data: [T; N], // the backing array
    len: usize, // how many slots are used
}

impl<T: Copy + From<u8>, const N: usize> Buf<T, N> { // From<u8> lets us push bytes as u16 too
    const fn new(zero: T) -> Self { // start filled with a zero element (0u8 or 0u16)
        Buf { data: [zero; N], len: 0 } // empty buffer
    }
    fn push(&mut self, items: &[T]) { // append items, silently dropping past the capacity
        for &item in items {
            if self.len < N { // only while there's room
                self.data[self.len] = item; // store at the end
                self.len += 1; // and count it
            }
        }
    }
    fn push_ascii(&mut self, s: &[u8]) { // append ASCII bytes, widened to T (u8 or u16)
        for &b in s {
            self.push(&[T::from(b)]); // widen and reuse push
        }
    }
    fn push_int(&mut self, n: i32) { // append n in decimal digits
        let (mut digits, mut i, mut v) = ([0u8; 11], 11, n.unsigned_abs()); // digit workspace, write index, working value
        loop {
            i -= 1; // fill the buffer from the right
            digits[i] = b'0' + (v % 10) as u8; // lowest digit as ASCII
            v /= 10; // drop that digit
            if v == 0 { // nothing left
                break;
            }
        }
        if n < 0 { // negative: a minus sign first
            self.push_ascii(b"-");
        }
        self.push_ascii(&digits[i..]); // then the digits we wrote
    }
    fn full(&self) -> bool { // did a push get dropped?
        self.len >= N
    }
    #[cfg(test)] // only tests need to read the contents back out
    fn as_slice(&self) -> &[T] {
        &self.data[..self.len] // the used part
    }
}

// core has no float rounding without libm, and these only need whole pixels.
fn floor(x: f64) -> f64 { // round towards negative infinity
    let t = x as i64 as f64; // truncate to an integer
    if t > x { t - 1.0 } else { t } // truncation rounded up: step back down
}

fn ceil(x: f64) -> f64 { // round towards positive infinity
    let t = x as i64 as f64; // truncate to an integer
    if t < x { t + 1.0 } else { t } // truncation rounded down: step back up
}

fn abs(x: f64) -> f64 { // absolute value
    if x < 0.0 { -x } else { x }
}

// ---- Cloudflare clef-flash: picks which way to send the ball back (Engine.java's ATTACK) ----

const SHOT_NONE: u8 = 0; // no answer, an error, or "straight": keep the defensive spot
const SHOT_UP: u8 = 1; // model says: aim the ball upward
const SHOT_DOWN: u8 = 2; // model says: aim the ball downward

struct Api {
    session: H, // WinHTTP session handle (reused for every request)
    connect: H, // connection to api.cloudflare.com (TLS reused too)
    path: Buf<u16, 160>, // request path, UTF-16, built once
    headers: Buf<u16, 320>, // request headers, UTF-16, built once
    body: Buf<u8, 1024>, // JSON body, rebuilt per request
    skip_revocation: bool // after a revocation-check failure, retry without it
}

static API: Global<Api> = Global(UnsafeCell::new(Api { // the one shared Api, zero-initialised
    session: null_mut(), // no session yet
    connect: null_mut(), // no connection yet
    path: Buf::new(0), // empty path buffer
    headers: Buf::new(0), // empty header buffer
    body: Buf::new(0), // empty body buffer
    skip_revocation: false, // start by checking revocation
}));
static FILE: Global<[u8; 4096]> = Global(UnsafeCell::new([0; 4096])); // scratch space for the .env file's bytes
static REPLY: Global<[u8; 4096]> = Global(UnsafeCell::new([0; 4096])); // scratch space for the model's reply bytes

fn trim(mut s: &[u8]) -> &[u8] { // strip spaces, tabs and CRs from both ends of a byte slice
    while let [b' ' | b'\t' | b'\r', rest @ ..] = s { // leading whitespace?
        s = rest; // drop the first byte
    }
    while let [rest @ .., b' ' | b'\t' | b'\r'] = s { // trailing whitespace?
        s = rest; // drop the last byte
    }
    s // the trimmed slice
}

// CLOUDFLARE_CLEF_API_KEY / CLOUDFLARE_ACCOUNT_ID from a .env file's text.
fn parse_env(text: &[u8]) -> Option<(&[u8], &[u8])> { // returns (key, account) slices into `text`
    let (mut key, mut account): (&[u8], &[u8]) = (&[], &[]); // not found yet
    for line in text.split(|&b| b == b'\n') { // one line at a time
        if let Some(eq) = line.iter().position(|&b| b == b'=') { // does it look like NAME=value?
            match trim(&line[..eq]) { // the name, without surrounding spaces
                b"CLOUDFLARE_CLEF_API_KEY" => key = trim(&line[eq + 1..]), // the key value
                b"CLOUDFLARE_ACCOUNT_ID" => account = trim(&line[eq + 1..]), // the account id
                _ => {} // some other variable: ignore
            }
        }
    }
    let printable = |s: &[u8]| s.iter().all(|&b| (0x21..0x7F).contains(&b)); // no header injection
    (!key.is_empty() && !account.is_empty() && printable(key) && printable(account)).then_some((key, account)) // both found and printable
}

// Read .env from the working directory, else next to the exe. No keys = defend only.
unsafe fn read_env() -> Option<&'static [u8]> {
    unsafe {
        let mut file = CreateFileW(wz!(".env"), GENERIC_READ, FILE_SHARE_READ, null(), OPEN_EXISTING, 0, null_mut()); // try ".env" in the cwd
        if file == INVALID_HANDLE { // not there?
            let mut path = [0u16; 300]; // room for a full exe path
            let len = GetModuleFileNameW(null_mut(), path.as_mut_ptr(), 260) as usize; // get the exe's own path
            let slash = path[..len.min(260)].iter().rposition(|&c| c == b'\\' as u16)?; // last backslash: stop if there is none
            path[slash + 1..slash + 5].copy_from_slice(w!(".env")); // overwrite the file name with ".env"
            path[slash + 5] = 0; // NUL-terminate it
            file = CreateFileW(path.as_ptr(), GENERIC_READ, FILE_SHARE_READ, null(), OPEN_EXISTING, 0, null_mut()); // try the exe folder
            if file == INVALID_HANDLE { // still nothing
                return None; // no keys, the AI stays defensive
            }
        }
        let buf = FILE.get(); // the shared 4 KB scratch buffer
        let mut read = 0; // how many bytes ReadFile read
        let ok = ReadFile(file, buf.as_mut_ptr(), buf.len() as u32, &mut read, null_mut()); // read the whole file
        CloseHandle(file); // done with the file
        (ok != 0).then(|| &buf[..(read as usize).min(buf.len())]) // the bytes read, capped to the buffer
    }
}

// One WinHTTP session and connection for the whole game: after the first request, later
// ones reuse the open TLS connection instead of a new process + handshake each time.
unsafe fn open_api() -> bool { // returns true when keys were found and the connection opened
    let Some((key, account)) = (unsafe { read_env() }).and_then(parse_env) else { return false }; // no valid .env: no AI
    let api = API.get();
    api.path.push_ascii(b"/client/v4/accounts/"); // build "/accounts/<id>/ai/run/@cf/cloudflare/clef-flash"
    api.path.push_ascii(account); // the account id from .env
    api.path.push_ascii(b"/ai/run/@cf/cloudflare/clef-flash\0"); // the model endpoint, NUL-terminated
    api.headers.push_ascii(b"Authorization: Bearer "); // auth header prefix
    api.headers.push_ascii(key); // the API key from .env
    api.headers.push_ascii(b"\r\nContent-Type: application/json\0"); // we're sending JSON, NUL-terminated
    if api.path.full() || api.headers.full() { // keys were absurdly long and got cut
        return false; // better to disable the AI than send a broken request
    }
    unsafe {
        api.session = WinHttpOpen(wz!("Pongie"), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null(), null(), 0); // open a session, agent "Pongie", system proxy
        if api.session.is_null() { // failed
            return false;
        }
        WinHttpSetTimeouts(api.session, 10_000, 10_000, 10_000, 10_000); // 10 s on every stage
        api.connect = WinHttpConnect(api.session, wz!("api.cloudflare.com"), 443, 0); // HTTPS port 443
        !api.connect.is_null() // success iff we got a connection handle
    }
}

fn build_body(body: &mut Buf<u8, 1024>, height: i32, hit_y: i32, player_middle: i32) { // the question to the model, as JSON
    body.len = 0; // start the JSON over
    body.push_ascii(br#"{"model":"clef-flash","state":"Pong. The screen is "#); // model name + start of the state text
    body.push_int(height); // how tall the screen is
    body.push_ascii(br#" tall, y=0 is the top. You are the right paddle and you are about to hit the ball back at y="#); // the situation
    body.push_int(hit_y); // where the paddle will hit the ball
    body.push_ascii(br#". The other player's paddle is on the left side, centred at y="#); // the opponent's position
    body.push_int(player_middle); // their paddle's middle
    body.push_ascii(concat!( // the question definition, concatenated from raw strings
        r#". The ball bounces off the top and bottom walls.","questions":{"shot":{"#, // closing state text, then the "shot" question
        r#""type":"choice","instructions":"Which way should you send the ball so it is "#, // a choice question
        r#"hardest for the other player to reach? Aim away from where they are.","criteria":{"#, // the instruction, then the answer options
        r#""up":"Send the ball upward, towards y=0","straight":"Send the ball straight across","#, // option descriptions
        r#""down":"Send the ball downward, towards the bottom"}}}}"# // last option, then close all the JSON braces
    ).as_bytes());
}

// Pull "down" out of ..."choice":"down",... Keeps scanning like a regex would: the reply
// also contains "type":"choice" before the answer.
fn parse_choice(reply: &[u8]) -> u8 { // extract the model's answer word from the raw JSON
    const KEY: &[u8] = b"\"choice\""; // the field we're looking for
    let skip_space = |s: &[u8], mut i: usize| { // helper: index of the next non-space byte
        while matches!(s.get(i), Some(b' ' | b'\t' | b'\r' | b'\n')) { // while there's whitespace at i
            i += 1;
        }
        i
    };
    for at in 0..reply.len() { // scan every position, like a regex search
        if !reply[at..].starts_with(KEY) { // no "choice" here
            continue;
        }
        let mut i = skip_space(reply, at + KEY.len()); // skip spaces after "choice"
        if reply.get(i) != Some(&b':') { // must be followed by a colon
            continue;
        }
        i = skip_space(reply, i + 1); // skip spaces after the colon
        if reply.get(i) != Some(&b'"') { // must open a quoted string
            continue;
        }
        let start = i + 1; // the answer word starts here
        let word = reply[start..].iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'_').count(); // letters/digits/underscores = the word's length
        if word > 0 && reply.get(start + word) == Some(&b'"') { // non-empty and properly closed
            return match &reply[start..start + word] { // map the word to a shot constant
                b"up" => SHOT_UP,
                b"down" => SHOT_DOWN,
                _ => SHOT_NONE, // "straight" or anything else: keep defending
            };
        }
    }
    SHOT_NONE // no usable answer anywhere in the reply
}

unsafe fn post_request() -> H { // send the request, return the request handle (null = failed)
    let api = API.get();
    // Best-effort revocation, like curl's --ssl-revoke-best-effort: school proxies re-sign HTTPS
    // with a CA whose revocation list is unreachable, so retry once without the check. A revoked
    // certificate still fails.
    for _ in 0..2 { // two attempts: normal, then without revocation checking
        unsafe {
            let request = WinHttpOpenRequest(api.connect, wz!("POST"), api.path.data.as_ptr(), null(), null(), // a POST to the saved path, over TLS
                                             null(), WINHTTP_FLAG_SECURE);
            if request.is_null() { // couldn't even build the request
                return null_mut();
            }
            if !api.skip_revocation { // first attempt: full certificate checking
                let on = WINHTTP_ENABLE_SSL_REVOCATION;
                WinHttpSetOption(request, WINHTTP_OPTION_ENABLE_FEATURE, (&raw const on).cast(), 4); // turn revocation on (4 = size of a u32)
            }
            let len = api.body.len as u32; // body size in bytes
            if WinHttpSendRequest(request, api.headers.data.as_ptr(), u32::MAX, api.body.data.as_ptr().cast(), len, len, 0) != 0 // send it (-1 length = headers are NUL-terminated)
                && WinHttpReceiveResponse(request, null_mut()) != 0 // and get the response headers
            {
                return request; // success: caller reads the body, then closes the handle
            }
            let error = GetLastError(); // why did it fail?
            WinHttpCloseHandle(request); // clean up the failed request
            if error != ERROR_WINHTTP_SECURE_CERT_REV_FAILED || api.skip_revocation { // any other error, or already retried
                return null_mut(); // give up
            }
            api.skip_revocation = true; // next loop: same request, minus the revocation check
        }
    }
    null_mut() // unreachable in practice
}

// One request: the answer, or SHOT_NONE for no answer, an HTTP error or "straight".
fn fetch_shot() -> u8 { // the full request/response cycle, synchronous
    unsafe {
        let request = post_request(); // send the question
        if request.is_null() { // network failure
            return SHOT_NONE;
        }
        let (mut status, mut size) = (0u32, 4u32); // status code and its buffer size
        WinHttpQueryHeaders(request, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, null(), // ask for the status code as a number
                            (&raw mut status).cast(), &mut size, null_mut());
        let reply = REPLY.get(); // the shared reply buffer
        let mut len = 0; // bytes read so far
        while len < reply.len() { // until the buffer is full
            let mut read = 0; // bytes in this chunk
            let chunk = &mut reply[len..]; // the free part of the buffer
            if WinHttpReadData(request, chunk.as_mut_ptr().cast(), chunk.len() as u32, &mut read) == 0 || read == 0 { // error or clean end of stream
                break;
            }
            len += read as usize; // got some bytes
        }
        WinHttpCloseHandle(request); // done with the request handle
        if (200..300).contains(&status) { parse_choice(&reply[..len.min(reply.len())]) } else { SHOT_NONE } // 2xx: parse the answer, else no answer
    }
}

// Worker thread: one request, then post the answer back to the window and exit.
unsafe extern "system" fn ask_model(approach: H) -> u32 { // runs on its own thread; param = the approach number
    let shot = fetch_shot(); // blocking network call
    unsafe { PostMessageW(game().window, WM_APP_SHOT, approach as usize, shot as isize) }; // hand the answer to the window thread
    0 // thread exit code
}

// ---- Game ----

#[derive(Clone, Copy, PartialEq)] // comparable and copyable
enum State {
    Countdown, // showing 3, 2, 1
    Playing, // the rally
    GameOver, // the ball got past someone
}

#[derive(Clone, Copy, PartialEq)]
struct Paddle {
    x: f64, // left edge position
    y: f64, // top edge position
}

#[derive(Clone, Copy)]
struct Ball {
    x: f64, // left edge position
    y: f64, // top edge position
    speed_x: f64, // pixels per frame horizontally
    speed_y: f64, // pixels per frame vertically
}

struct Game {
    window: H, // the window handle (0 until the window exists)
    width: i32, // client area width in pixels
    height: i32, // client area height in pixels
    player: Paddle, // your paddle (left side)
    opponent: Paddle, // the AI paddle (right side)
    ball: Ball, // the ball
    opponent_target_y: f64, // where the AI paddle is heading
    state: State, // countdown / playing / game over
    timer: i32, // frames left in the current countdown or game-over pause
    message_timer: i32, // frames left before the centre message disappears
    message: &'static [u16], // the centre message text, UTF-16 (empty = none)
    moving_up: bool, // up key held?
    moving_down: bool, // down key held?
    fullscreen: bool, // currently fullscreen?
    windowed_style: isize, // style bits saved when going fullscreen
    font: H, // the big bold score/announcement font
    back_dc: H, // off-screen device context (double buffering)
    back_bitmap: H, // the off-screen pixels
    original_bitmap: H, // the 1x1 placeholder the dc started with
    dirty_region: H, // region describing what needs redrawing
    has_api: bool, // did we get API keys and a connection?
    approach: u32, // bumped every time the ball heads at the opponent
    asking: bool, // a model request is in flight (one at a time, like Engine.java)
    landing_y: f64, // where the ball will meet the AI paddle
}

static GAME: Global<Game> = Global(UnsafeCell::new(Game::new())); // the one shared Game

// Window calls like SetWindowPos send WM_SIZE straight back into window_proc, so never
// keep this reference across one of those calls.
fn game() -> &'static mut Game { // a mutable reference to the global game
    GAME.get() // the caller promises not to hold it across re-entrant calls
}

// Same order as Paddle.java: top first, then bottom (bottom wins on a tiny window).
fn keep_on_screen(y: f64, screen_height: i32) -> f64 { // clamp a paddle's y inside the window
    y.max(0.0).min((screen_height - PADDLE_HEIGHT) as f64) // at least 0, at most bottom edge
}

fn overlaps(b: &Ball, p: &Paddle) -> bool { // do the ball's and paddle's rectangles intersect?
    b.x < p.x + PADDLE_WIDTH as f64 && b.x + BALL_SIZE as f64 > p.x // x ranges overlap
        && b.y < p.y + PADDLE_HEIGHT as f64 && b.y + BALL_SIZE as f64 > p.y // y ranges overlap
}

impl Game {
    const fn new() -> Self { // a game with everything at its starting value (const: runs in a static)
        Game {
            window: null_mut(), // no window yet
            width: 800, // default client size
            height: 600,
            player: Paddle { x: 20.0, y: 0.0 }, // left edge, y fixed by reset()
            opponent: Paddle { x: 765.0, y: 0.0 }, // right edge, y fixed by reset()
            ball: Ball { x: 0.0, y: 0.0, speed_x: START_SPEED, speed_y: START_SPEED }, // ball fixed by reset()
            opponent_target_y: 300.0, // AI heads for the middle until told otherwise
            state: State::Countdown, // the opening 3-2-1
            timer: 3 * FPS, // three seconds of countdown
            message_timer: 0, // no temporary message showing
            message: w!("3"), // first countdown number
            moving_up: false, // no keys held
            moving_down: false,
            fullscreen: false, // start windowed
            windowed_style: 0, // nothing saved yet
            font: null_mut(), // fonts created in run()
            back_dc: null_mut(), // back buffer created on first WM_SIZE
            back_bitmap: null_mut(),
            original_bitmap: null_mut(),
            dirty_region: null_mut(),
            has_api: false, // API opened in run()
            approach: 0, // no approaches yet
            asking: false, // nothing in flight
            landing_y: 300.0, // placeholder
        }
    }

    // Integer maths on purpose: matches Java's int positions exactly.
    fn reset(&mut self) { // put paddles and ball back at their starting positions
        let paddle_y = (self.height / 2 - PADDLE_HEIGHT / 2) as f64; // vertically centred paddle
        self.player = Paddle { x: 20.0, y: paddle_y }; // left side
        self.opponent = Paddle { x: (self.width - 35) as f64, y: paddle_y }; // right side
        self.ball = Ball { // centre of the screen
            x: (self.width / 2 - BALL_SIZE / 2) as f64,
            y: (self.height / 2 - BALL_SIZE / 2) as f64,
            speed_x: START_SPEED, // base speed again
            speed_y: START_SPEED,
        };
        if unsafe { GetTickCount64() } & 1 == 1 { // milliseconds since boot, lowest bit
            self.ball.speed_x = -self.ball.speed_x; // odd: serve to the left instead
        }
    }

    // Plan once per approach, like the Java engine thread, not every frame.
    // DEFEND: head for the landing spot now. ATTACK: ask the model (off-thread) which way to aim.
    fn aim(&mut self) { // decide where the AI paddle should go
        self.approach = self.approach.wrapping_add(1); // a new approach id (wraps silently at u32::MAX)
        if self.ball.speed_x <= 0.0 { // ball moving away from the AI
            self.opponent_target_y = (self.height / 2) as f64; // wait in the middle
            return;
        }
        self.landing_y = self.predict_landing_y(); // where will the ball arrive?
        self.opponent_target_y = self.landing_y; // defensive plan: be there
        if !self.has_api || self.asking { // no AI configured, or a question is already in flight
            return;
        }
        let player_middle = self.player.y as i32 + PADDLE_HEIGHT / 2; // the opponent's paddle centre
        build_body(&mut API.get().body, self.height, self.landing_y as i32, player_middle); // write the question
        // ponytail: a short-lived thread per approach (a few per rally); the connection is what's reused.
        let thread = unsafe {
            CreateThread(null(), 64 * 1024, ask_model, self.approach as usize as H, // run ask_model on a 64 KB stack, param = approach id
                         STACK_SIZE_PARAM_IS_A_RESERVATION, null_mut()) // stack size is a reservation; don't need the thread id
        };
        if !thread.is_null() { // thread started
            self.asking = true; // exactly one request in flight
            unsafe { CloseHandle(thread) }; // we don't wait for it: the answer arrives as a message
        }
    }

    // Apply a model answer if it is for this approach and the ball is still far enough away.
    fn take_shot(&mut self, approach: u32, shot: u8) { // called from WM_APP_SHOT
        self.asking = false; // the request finished either way
        let frames_left = (self.opponent.x - (self.ball.x + BALL_SIZE as f64)) / self.ball.speed_x; // how many frames until contact
        if approach != self.approach || self.ball.speed_x <= 0.0 || frames_left <= 15.0 { // stale answer, ball heading away, or too late to move
            return; // ignore it, keep the defensive position
        }
        match shot {
            SHOT_UP => self.opponent_target_y = self.landing_y + AIM_SHIFT, // hit its top part
            SHOT_DOWN => self.opponent_target_y = self.landing_y - AIM_SHIFT, // hit its bottom part
            _ => {} // straight or none: already defending
        }
    }

    fn predict_landing_y(&self) -> f64 { // simulate the ball's flight to the AI paddle
        let (mut x, mut y, mut speed_y) = (self.ball.x, self.ball.y, self.ball.speed_y); // a copy of the ball's state
        while x + (BALL_SIZE as f64) < self.opponent.x { // until it reaches the paddle
            x += self.ball.speed_x; // step horizontally
            y += speed_y; // and vertically
            if y <= 0.0 && speed_y < 0.0 { // past the top wall, moving up
                speed_y = -speed_y; // bounce down
            }
            if y + BALL_SIZE as f64 >= self.height as f64 && speed_y > 0.0 { // past the bottom wall, moving down
                speed_y = -speed_y; // bounce up
            }
        }
        (y as i32 + BALL_SIZE / 2) as f64 // Java truncates to int here
    }

    fn hit_paddle(&mut self, paddle: Paddle) { // the ball hit a paddle: reflect and speed up
        let ball_middle = self.ball.y + BALL_SIZE as f64 / 2.0; // ball's vertical centre
        let paddle_middle = paddle.y + PADDLE_HEIGHT as f64 / 2.0; // paddle's vertical centre
        let hit_spot = ((ball_middle - paddle_middle) / (PADDLE_HEIGHT as f64 / 2.0)).clamp(-1.0, 1.0); // -1..1: where on the paddle it hit
        self.ball.speed_x = -self.ball.speed_x * SPEED_UP; // reverse horizontally, 5% faster
        self.ball.speed_y = hit_spot * abs(self.ball.speed_x); // steeper the further from the middle
        self.aim(); // the ball now heads at the AI: plan a new approach
    }

    fn invalidate_sprite(&self, x: f64, y: f64, w: i32, h: i32) { // mark a sprite's rectangle dirty
        let area = Rect {
            left: floor(x) as i32 - 1, // one pixel of margin for anti-aliasing
            top: floor(y) as i32 - 1,
            right: ceil(x + w as f64) as i32 + 1,
            bottom: ceil(y + h as f64) as i32 + 1,
        };
        unsafe { InvalidateRect(self.window, &area, 0) }; // queue a repaint of just this area
    }

    // Old and new spot of anything that moved; a paddle standing still isn't redrawn.
    fn invalidate_paddle(&self, old: Paddle, new: Paddle) { // repaint both ends of a paddle's move
        if old != new { // only if it actually moved
            self.invalidate_sprite(old.x, old.y, PADDLE_WIDTH, PADDLE_HEIGHT); // erase where it was
            self.invalidate_sprite(new.x, new.y, PADDLE_WIDTH, PADDLE_HEIGHT); // draw where it is
        }
    }

    // Returns true when the whole window needs repainting (the message changed).
    fn update(&mut self) -> bool { // one frame of game logic
        if self.state == State::Playing { // the rally is live
            let (old_player, old_opponent, old_ball) = (self.player, self.opponent, self.ball); // remember where everything was

            let speed = abs(self.ball.speed_x); // current ball speed sets everyone's pace
            let paddle_speed = (speed + 0.5) as i64 as f64; // Java paddles move whole pixels (round)
            if self.moving_up { // up key held
                self.player.y -= paddle_speed; // paddle up
            }
            if self.moving_down { // down key held
                self.player.y += paddle_speed; // paddle down
            }
            self.player.y = keep_on_screen(self.player.y, self.height); // clamp inside the window

            let opponent_middle = self.opponent.y + PADDLE_HEIGHT as f64 / 2.0; // AI paddle's centre
            if opponent_middle > self.opponent_target_y + speed { // above its target
                self.opponent.y -= paddle_speed; // move down towards it
            }
            if opponent_middle < self.opponent_target_y - speed { // below its target
                self.opponent.y += paddle_speed; // move up towards it
            }
            self.opponent.y = keep_on_screen(self.opponent.y, self.height); // clamp inside the window

            self.ball.x += self.ball.speed_x; // ball moves horizontally
            self.ball.y += self.ball.speed_y; // and vertically
            if self.ball.y <= 0.0 && self.ball.speed_y < 0.0 { // past the top, moving up
                self.ball.speed_y = -self.ball.speed_y; // bounce down
            }
            if self.ball.y + BALL_SIZE as f64 >= self.height as f64 && self.ball.speed_y > 0.0 { // past the bottom, moving down
                self.ball.speed_y = -self.ball.speed_y; // bounce up
            }

            if overlaps(&self.ball, &self.player) && self.ball.speed_x < 0.0 && self.ball.x > self.player.x { // ball reached your paddle from the right
                self.hit_paddle(self.player); // hit it back
            }
            if overlaps(&self.ball, &self.opponent) && self.ball.speed_x > 0.0 && self.ball.x < self.opponent.x { // ball reached the AI's paddle from the left
                self.hit_paddle(self.opponent); // hit it back
            }

            if self.ball.x <= 0.0 || self.ball.x + BALL_SIZE as f64 >= self.width as f64 { // ball past either side: point over
                self.reset(); // everything back to the middle
                self.state = State::GameOver;
                self.message = w!("GAME OVER");
                self.timer = 3 * FPS; // three second pause
                return true; // full repaint: the message changed
            }
            self.invalidate_paddle(old_player, self.player); // repaint your paddle only where it moved
            self.invalidate_paddle(old_opponent, self.opponent); // same for the AI's
            self.invalidate_sprite(old_ball.x, old_ball.y, BALL_SIZE, BALL_SIZE); // erase the ball's old spot
            self.invalidate_sprite(self.ball.x, self.ball.y, BALL_SIZE, BALL_SIZE); // draw its new one
            return self.tick_message(); // and maybe the message expired
        }

        self.timer -= 1; // countdown or game-over pause: one frame less
        if self.state == State::GameOver { // mid game-over pause
            if self.timer == 0 { // pause over
                self.state = State::Countdown; // new rally starting
                self.message = w!("3");
                self.timer = 3 * FPS;
                return true;
            }
            return false; // nothing changed this frame
        }

        match self.timer { // in the countdown: change the big number
            t if t == 2 * FPS => self.message = w!("2"), // one second in: "2"
            t if t == FPS => self.message = w!("1"), // two seconds in: "1"
            0 => { // countdown over: play!
                self.state = State::Playing;
                self.message = w!("GO!");
                self.message_timer = FPS; // hide "GO!" after a second
                self.aim(); // the ball is heading at the AI: plan the first approach
            }
            _ => return self.tick_message(), // between numbers: nothing to do
        }
        true // the message changed: full repaint
    }

    // Hides "GO!" after a second.
    fn tick_message(&mut self) -> bool { // countdown for the temporary message
        if self.message_timer > 0 { // a message is showing
            self.message_timer -= 1;
            if self.message_timer == 0 { // time's up
                self.message = &[]; // clear it
                return true; // repaint without it
            }
        }
        false // nothing changed
    }

    fn paint(&self, dc: H) { // draw the whole game into a dc
        let area = Rect { left: 0, top: 0, right: self.width, bottom: self.height }; // the whole client area
        // ponytail: GAME OVER overlay (black at alpha 180) pre-multiplied into the
        // sprite colours; the background is already black so no AlphaBlend needed.
        let keep = if self.state == State::GameOver { 255 - 180 } else { 255 }; // dim everything on game over
        let shade = |r: u32, g: u32, b: u32| rgb(r * keep / 255, g * keep / 255, b * keep / 255); // a colour, darkened if game over
        unsafe {
            FillRect(dc, &area, GetStockObject(BLACK_BRUSH)); // black background

            let brush = GetStockObject(DC_BRUSH); // the reusable colour brush
            SetDCBrushColor(dc, shade(0, 0, 255)); // Java's Color.BLUE
            for p in [self.player, self.opponent] { // both paddles
                let (x, y) = (p.x as i32, p.y as i32); // to whole pixels
                let r = Rect { left: x, top: y, right: x + PADDLE_WIDTH, bottom: y + PADDLE_HEIGHT }; // the paddle's rectangle
                FillRect(dc, &r, brush); // draw it
            }

            SetDCBrushColor(dc, shade(255, 0, 0)); // red ball
            SetDCPenColor(dc, shade(255, 0, 0)); // red outline too
            let old_brush = SelectObject(dc, brush); // select brush and pen, remember the old ones
            let old_pen = SelectObject(dc, GetStockObject(DC_PEN));
            let (x, y) = (self.ball.x as i32, self.ball.y as i32); // to whole pixels
            Ellipse(dc, x, y, x + BALL_SIZE, y + BALL_SIZE); // the ball as a circle
            SelectObject(dc, old_pen); // put the old objects back
            SelectObject(dc, old_brush);

            if !self.message.is_empty() { // a centre message to show
                let old_font = SelectObject(dc, self.font); // the big bold font
                SetTextColor(dc, rgb(255, 255, 255)); // white text
                SetBkMode(dc, TRANSPARENT); // no box behind the letters
                SetTextAlign(dc, TA_CENTER | TA_BASELINE); // Java: centred, baseline at mid-height
                TextOutW(dc, self.width / 2, self.height / 2, self.message.as_ptr(), self.message.len() as i32); // dead centre
                SelectObject(dc, old_font); // back to the default font
            }
        }
    }

    fn release_back_buffer(&mut self) { // free the off-screen bitmap and dc
        if self.back_dc.is_null() { // nothing to free
            return;
        }
        unsafe {
            if !self.original_bitmap.is_null() { // put the dc's original bitmap back
                SelectObject(self.back_dc, self.original_bitmap);
            }
            if !self.back_bitmap.is_null() { // free our bitmap
                DeleteObject(self.back_bitmap);
            }
            DeleteDC(self.back_dc); // free the dc
        }
        self.back_dc = null_mut(); // everything back to "none"
        self.back_bitmap = null_mut();
        self.original_bitmap = null_mut();
    }

    fn create_back_buffer(&mut self, window: H) { // (re)build the off-screen buffer at the current size
        self.release_back_buffer(); // drop any old one first
        unsafe {
            let screen = GetDC(window); // a dc to be compatible with
            self.back_dc = CreateCompatibleDC(screen); // off-screen dc
            self.back_bitmap = CreateCompatibleBitmap(screen, self.width, self.height); // off-screen bitmap, client-sized
            ReleaseDC(window, screen); // done with the screen dc
            if self.back_dc.is_null() || self.back_bitmap.is_null() { // creation failed
                self.release_back_buffer(); // don't keep a half-built buffer
                return; // paint will be skipped until a resize succeeds
            }
            self.original_bitmap = SelectObject(self.back_dc, self.back_bitmap); // point the dc at our bitmap
        }
    }
}

fn toggle_fullscreen() { // F11: switch between fullscreen and a centred 800x600 window
    let (window, fullscreen) = (game().window, game().fullscreen); // snapshot the bits we need
    unsafe {
        let mut monitor: MonitorInfo = core::mem::zeroed(); // all zero, as Win32 requires
        monitor.size = size_of::<MonitorInfo>() as u32; // tell Windows the struct version
        GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &mut monitor); // this monitor's rectangles
        if !fullscreen { // going fullscreen
            let style = GetWindowLongPtrW(window, GWL_STYLE); // remember the windowed style
            game().windowed_style = style;
            game().fullscreen = true;
            let m = monitor.monitor; // the monitor's full rectangle
            SetWindowLongPtrW(window, GWL_STYLE, style & !(WS_OVERLAPPEDWINDOW as isize)); // strip title bar and borders
            SetWindowPos(window, null_mut(), m.left, m.top, m.right - m.left, m.bottom - m.top, // cover the whole monitor
                         SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        } else {
            // Like Java's pack(): a normal (not maximized) window with an 800x600 game area, centred.
            game().fullscreen = false;
            let style = game().windowed_style & !(WS_MAXIMIZE as isize); // restore style, minus maximized
            let mut r = Rect { left: 0, top: 0, right: 800, bottom: 600 }; // the client area we want
            AdjustWindowRectEx(&mut r, style as u32, 0, 0); // grow it to the full window size (title bar etc.)
            let (w, h) = (r.right - r.left, r.bottom - r.top); // the outer size
            let work = monitor.work; // monitor minus the taskbar
            SetWindowLongPtrW(window, GWL_STYLE, style); // put the borders back
            SetWindowPos(window, null_mut(), work.left + (work.right - work.left - w) / 2, // centred in the work area
                         work.top + (work.bottom - work.top - h) / 2, w, h,
                         SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        }
    }
}

unsafe extern "system" fn window_proc(window: H, message: u32, wparam: usize, lparam: isize) -> isize { // Win32 calls this for every window event
    let up = wparam == 'E' as usize || wparam == VK_UP; // the "up" keys (E or the up arrow)
    let down = wparam == 'D' as usize || wparam == VK_DOWN; // the "down" keys (D or the down arrow)
    unsafe {
        match message {
            WM_SIZE => { // the window was resized
                let g = game();
                g.width = (lparam & 0xFFFF).max(1) as i32; // low 16 bits: new width, at least 1
                g.height = ((lparam >> 16) & 0xFFFF).max(1) as i32; // high 16 bits: new height
                g.create_back_buffer(window); // rebuild the off-screen buffer at the new size
                g.opponent.x = (g.width - 35) as f64; // keep it on the right edge
                if g.state != State::Playing { // not mid-rally
                    g.reset(); // centre everything again
                }
                InvalidateRect(window, null(), 0); // repaint everything
                0
            }
            WM_ERASEBKGND => 1, // we erase the background ourselves in paint: skip the flicker
            WM_PAINT => { // something needs drawing
                let g = game();
                let region_type = GetUpdateRgn(window, g.dirty_region, 0); // what exactly is dirty?
                let mut ps: PaintStruct = core::mem::zeroed(); // paint info, zeroed as Win32 requires
                let screen = BeginPaint(window, &mut ps); // start painting the real window
                if !g.back_dc.is_null() { // back buffer exists
                    if region_type > NULLREGION { // and we know which part is dirty
                        SelectClipRgn(g.back_dc, g.dirty_region); // limit drawing to it
                    }
                    g.paint(g.back_dc); // draw the game off-screen
                    let d = ps.paint; // the dirty rectangle
                    BitBlt(screen, d.left, d.top, d.right - d.left, d.bottom - d.top, // copy just that part to the screen
                           g.back_dc, d.left, d.top, SRCCOPY);
                    SelectClipRgn(g.back_dc, null_mut()); // remove the clip limit again
                }
                EndPaint(window, &ps); // done painting
                0
            }
            WM_APP_SHOT => { // a model answer arrived
                game().take_shot(wparam as u32, lparam as u8); // apply it if still relevant
                0
            }
            WM_KEYDOWN => {
                if lparam & (1 << 30) == 0 && (wparam == VK_F11 || (wparam == VK_ESCAPE && game().fullscreen)) { // first press only (bit 30 = auto-repeat): F11, or Esc while fullscreen
                    toggle_fullscreen(); // switch display mode
                }
                let g = game();
                g.moving_up |= up; // register the held key
                g.moving_down |= down;
                0
            }
            WM_KEYUP => {
                let g = game();
                g.moving_up &= !up; // forget the released key
                g.moving_down &= !down;
                0
            }
            WM_KILLFOCUS => { // alt-tabbed away
                let g = game();
                g.moving_up = false; // stop the paddle drifting
                g.moving_down = false;
                0
            }
            WM_DESTROY => { // the window is closing
                game().release_back_buffer(); // free the off-screen buffer
                PostQuitMessage(0); // and end the main loop
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam), // everything else: Windows' default behaviour
        }
    }
}

unsafe fn run() { // create the window and pump frames until quit
    unsafe {
        let instance = GetModuleHandleW(null()); // this exe's module handle
        let class_name = wz!("PongRsWindow"); // our window class's name
        let class = WndClass {
            style: 0, // no class-wide style
            wnd_proc: window_proc, // our event callback
            cls_extra: 0, // no extra class bytes
            wnd_extra: 0, // no extra window bytes
            instance, // who owns the class
            icon: null_mut(), // default icon
            cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16), // standard arrow cursor
            background: GetStockObject(BLACK_BRUSH), // black window background
            menu_name: null(), // no menu
            class_name, // the name defined above
        };
        if RegisterClassW(&class) == 0 { // 0 = failed to register
            return; // can't open a window: give up silently
        }

        let g = game();
        // Negative height = em size, which is what Java's 72pt Arial means.
        g.font = CreateFontW(-72, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, // 72px tall, bold, no decorations
                             CLEARTYPE_QUALITY, 0, wz!("Arial")); // ClearType, default family, Arial
        g.dirty_region = CreateRectRgn(0, 0, 0, 0); // an empty region to track dirty areas
        g.has_api = open_api(); // try to set up the AI (true only with a valid .env)
        g.reset(); // paddles and ball to their starting spots
        let window = CreateWindowExW(0, class_name, wz!("Pongie (Rust)"), WS_OVERLAPPEDWINDOW, // a normal window titled "Pongie (Rust)"
                                     CW_USEDEFAULT, CW_USEDEFAULT, 800, 600, // default position, 800x600 client
                                     null_mut(), null_mut(), instance, null_mut()); // no parent, menu, or params
        if window.is_null() { // window creation failed
            return;
        }
        game().window = window; // the game can now invalidate and post messages
        ShowWindow(window, SW_MAXIMIZE); // show it maximized, like the Java version
        UpdateWindow(window); // force the first paint immediately

        // Sub-millisecond waits on Windows 10 1803+; plain ~15ms sleeps otherwise.
        let timer = CreateWaitableTimerExW(null(), null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS); // the frame timer (may fail: then we fall back)
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
                    MsgWaitForMultipleObjects(0, null(), 0, (wait_100ns / 10_000).max(1) as u32, QS_ALLINPUT); // no timer: coarse millisecond wait, at least 1
                }
            }

            // No TranslateMessage: nothing here reads WM_CHAR.
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 { // drain every pending message
                if msg.message == WM_QUIT { // quit was posted
                    break 'running; // leave the frame loop
                }
                DispatchMessageW(&msg); // otherwise run it through window_proc
            }

            QueryPerformanceCounter(&mut now);
            if now >= next_frame { // is it time for the next frame?
                if game().update() { // run one step; did anything worth repainting change?
                    InvalidateRect(window, null(), 0); // queue a full repaint
                }
                next_frame += frame; // schedule the next one
                if next_frame <= now { // we're badly behind
                    next_frame = now + frame; // fell behind: don't burst to catch up
                }
            }
        }
        // ExitProcess frees the font, region, timer and WinHTTP handles.
    }
}

// Entry point when there is no C runtime at all (msvc + /NODEFAULTLIB): we are the CRT.
#[cfg(all(not(test), target_env = "msvc"))]
#[unsafe(no_mangle)] // export it under the exact name the linker looks for
extern "system" fn mainCRTStartup() -> ! { // the exe's first function
    unsafe {
        run(); // play until quit
        ExitProcess(0) // exit with code 0
    }
}

// Entry point when a MinGW CRT is linked in: it already defines mainCRTStartup and
// calls WinMain for a GUI executable, so don't collide with it, just take over from it.
#[cfg(all(not(test), not(target_env = "msvc")))]
#[unsafe(no_mangle)]
extern "system" fn WinMain(_instance: H, _previous: H, _command_line: *const u8, _show: i32) -> i32 { // the CRT calls this for a GUI exe
    unsafe {
        run(); // play until quit
        ExitProcess(0) // exit with code 0 (never returns, so the i32 is unreachable)
    }
}

#[cfg(not(test))]
#[panic_handler] // no std, so we must define what a panic does
fn panic(_: &core::panic::PanicInfo) -> ! { // any panic: exit with code 1
    unsafe { ExitProcess(1) }
}

// The linker wants this marker whenever floating point is used without a C runtime.
#[cfg(not(test))]
#[unsafe(no_mangle)] // exported under the exact name the linker checks for
static _fltused: i32 = 0; // a dummy: the Microsoft linker just needs the symbol to exist

#[cfg(test)]
mod tests { // unit tests, run with `cargo test` (this build uses std normally)
    use super::*; // see everything in the file

    #[test]
    fn opponent_predicts_bounces_and_paddles_aim() { // the AI's maths
        let mut g = Game::new();
        g.opponent.x = 500.0; // put the AI paddle within reach of the test balls

        g.ball = Ball { x: 0.0, y: 100.0, speed_x: 10.0, speed_y: 0.0 }; // flying flat at y=100
        assert_eq!(g.predict_landing_y(), 115.0); // lands centred at 100 + 15

        // Up 5 steps to the top wall, bounce, then down 42 more: y = 420.
        g.ball = Ball { x: 0.0, y: 50.0, speed_x: 10.0, speed_y: -10.0 }; // flying up-left... at the top wall
        assert_eq!(g.predict_landing_y(), 435.0); // ends centred at 420 + 15

        let paddle = Paddle { x: 20.0, y: 100.0 };
        g.ball = Ball { x: 30.0, y: 135.0, speed_x: -10.0, speed_y: 3.0 }; // dead centre
        g.hit_paddle(paddle); // hit in the middle: straight back
        assert!((g.ball.speed_x - 10.5).abs() < 1e-9); // reversed and 5% faster
        assert_eq!(g.ball.speed_y, 0.0); // no vertical component

        g.ball = Ball { x: 30.0, y: 185.0, speed_x: -10.0, speed_y: 0.0 }; // bottom edge
        g.hit_paddle(paddle); // hit at the very bottom
        assert!((g.ball.speed_y - 10.5).abs() < 1e-9); // max deflection

        // A model answer for this approach shifts the aim; a stale one is ignored.
        g.ball = Ball { x: 0.0, y: 100.0, speed_x: 10.0, speed_y: 0.0 };
        g.aim(); // plans approach 1, target = landing spot
        g.take_shot(g.approach - 1, SHOT_UP); // an old approach number
        assert_eq!(g.opponent_target_y, 115.0); // ignored: still the landing spot
        g.take_shot(g.approach, SHOT_DOWN); // the current approach
        assert_eq!(g.opponent_target_y, 115.0 - AIM_SHIFT); // applied: aim shifted down
    }

    #[test]
    fn reads_env_and_the_models_choice_and_clamps_like_java() { // parsing helpers
        assert_eq!(parse_choice(br#"{"result":{"choice" : "down","confidence":0.4}}"#), SHOT_DOWN); // spaces around the colon
        assert_eq!(parse_choice(br#"{"errors":[]}"#), SHOT_NONE); // no answer in the reply
        let real = br#"{"result":{"model":"clef-flash","answers":{"shot":{"type":"choice","choice":"up","probabilities":{"up":0.27}}}}}"#; // a realistic reply
        assert_eq!(parse_choice(real), SHOT_UP); // finds the answer despite "type":"choice"
        assert_eq!(parse_choice(br#""choice":"straight""#), SHOT_NONE); // "straight" = keep defending
        assert_eq!(keep_on_screen(-5.0, 600), 0.0); // clamped to the top
        assert_eq!(keep_on_screen(550.0, 600), 500.0); // clamped to the bottom
        assert_eq!((floor(-1.5), ceil(-1.5), floor(2.0), ceil(2.25)), (-2.0, -1.0, 2.0, 3.0)); // rounding helpers

        let env = b"# keys\r\nCLOUDFLARE_ACCOUNT_ID = abc123\r\nCLOUDFLARE_CLEF_API_KEY=k-ey_9\r\n"; // a .env with spaces and CRLF line endings
        assert_eq!(parse_env(env), Some((&b"k-ey_9"[..], &b"abc123"[..]))); // both values found and trimmed
        assert_eq!(parse_env(b"CLOUDFLARE_CLEF_API_KEY=x\nCLOUDFLARE_ACCOUNT_ID=a b"), None); // a space in the id: rejected

        let mut body = Buf::<u8, 1024>::new(0);
        build_body(&mut body, 600, -450, 100); // a question with a negative hit_y
        let text = core::str::from_utf8(body.as_slice()).unwrap(); // the JSON as text
        assert!(!body.full() && text.contains("is 600 tall") && text.contains("y=-450.") && text.ends_with("}}}}")); // numbers in, JSON closed
        let json_braces = text.matches('{').count() == text.matches('}').count(); // balanced braces
        assert!(json_braces, "{text}"); // or fail showing the body
    }

    // Real network call with ../pong/.env: cargo test --release -- --ignored
    #[test]
    #[ignore] // only run on purpose: it hits the network
    fn live_model_answers() { // end-to-end test against the real API
        std::env::set_current_dir("../pong").unwrap(); // the .env lives in the Java project
        assert!(unsafe { open_api() }, "no keys in ../pong/.env"); // must find valid keys
        for _ in 0..2 { // the second request reuses the connection
            build_body(&mut API.get().body, 600, 450, 100); // a normal question
            REPLY.get().fill(0); // clear the reply buffer
            let started = std::time::Instant::now(); // time the round trip
            let shot = fetch_shot(); // the real request
            println!("shot {shot} in {:?}", started.elapsed()); // show the answer and timing
            let raw = String::from_utf8_lossy(&REPLY.get()[..]).trim_end_matches('\0').to_string(); // the raw reply as text
            assert!(raw.contains("\"choice\""), "no answer from clef-flash: {raw}"); // must contain an answer
        }
    }
}

// touch
