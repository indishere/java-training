// No Rust standard library and no C runtime: the exe imports only from DLLs that ship with
// Windows (raw-dylib, so no import libraries or Visual Studio are needed to link it).
// `cargo test` builds with std and ordinary import libraries (no dlltool needed).
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![windows_subsystem = "windows"]
#![cfg_attr(test, allow(dead_code))]

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ptr::{null, null_mut};

const FPS: i32 = 60;
const BALL_SIZE: i32 = 30;
const PADDLE_WIDTH: i32 = 15;
const PADDLE_HEIGHT: i32 = 100;
const START_SPEED: f64 = 7.0;
const SPEED_UP: f64 = 1.05;
const AIM_SHIFT: f64 = 35.0; // how far off the paddle's middle to hit when aiming up or down

// ---- Win32: only the calls this game makes, declared by hand so no crates are needed ----

type H = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct WndClass {
    style: u32,
    wnd_proc: unsafe extern "system" fn(H, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    instance: H,
    icon: H,
    cursor: H,
    background: H,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct Msg {
    hwnd: H,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: [i32; 2],
    private: u32,
}

#[repr(C)]
struct PaintStruct {
    hdc: H,
    erase: i32,
    paint: Rect,
    restore: i32,
    inc_update: i32,
    reserved: [u8; 32],
}

#[repr(C)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

#[cfg_attr(all(windows, not(test)), link(name = "user32", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "user32"))]
unsafe extern "system" {
    fn RegisterClassW(class: *const WndClass) -> u16;
    fn CreateWindowExW(ex_style: u32, class: *const u16, title: *const u16, style: u32,
                       x: i32, y: i32, w: i32, h: i32,
                       parent: H, menu: H, instance: H, param: H) -> H;
    fn DefWindowProcW(hwnd: H, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn ShowWindow(hwnd: H, cmd: i32) -> i32;
    fn UpdateWindow(hwnd: H) -> i32;
    fn PeekMessageW(msg: *mut Msg, hwnd: H, min: u32, max: u32, remove: u32) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn PostMessageW(hwnd: H, msg: u32, wparam: usize, lparam: isize) -> i32;
    fn PostQuitMessage(code: i32);
    fn MsgWaitForMultipleObjects(count: u32, handles: *const H, wait_all: i32, ms: u32, wake: u32) -> u32;
    fn InvalidateRect(hwnd: H, rect: *const Rect, erase: i32) -> i32;
    fn GetDC(hwnd: H) -> H;
    fn ReleaseDC(hwnd: H, dc: H) -> i32;
    fn BeginPaint(hwnd: H, ps: *mut PaintStruct) -> H;
    fn EndPaint(hwnd: H, ps: *const PaintStruct) -> i32;
    fn GetUpdateRgn(hwnd: H, rgn: H, erase: i32) -> i32;
    fn FillRect(dc: H, rect: *const Rect, brush: H) -> i32;
    fn LoadCursorW(instance: H, name: *const u16) -> H;
    fn GetWindowLongPtrW(hwnd: H, index: i32) -> isize;
    fn SetWindowLongPtrW(hwnd: H, index: i32, value: isize) -> isize;
    fn SetWindowPos(hwnd: H, after: H, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
    fn MonitorFromWindow(hwnd: H, flags: u32) -> H;
    fn GetMonitorInfoW(monitor: H, info: *mut MonitorInfo) -> i32;
    fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex_style: u32) -> i32;
}

#[cfg_attr(all(windows, not(test)), link(name = "gdi32", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "gdi32"))]
unsafe extern "system" {
    fn GetStockObject(index: i32) -> H;
    fn SetDCBrushColor(dc: H, color: u32) -> u32;
    fn SetDCPenColor(dc: H, color: u32) -> u32;
    fn SelectObject(dc: H, object: H) -> H;
    fn Ellipse(dc: H, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    fn SetTextColor(dc: H, color: u32) -> u32;
    fn SetBkMode(dc: H, mode: i32) -> i32;
    fn SetTextAlign(dc: H, align: u32) -> u32;
    fn TextOutW(dc: H, x: i32, y: i32, text: *const u16, len: i32) -> i32;
    fn CreateFontW(height: i32, width: i32, escapement: i32, orientation: i32, weight: i32,
                   italic: u32, underline: u32, strike_out: u32, charset: u32,
                   out_precision: u32, clip_precision: u32, quality: u32,
                   pitch_and_family: u32, face: *const u16) -> H;
    fn DeleteObject(object: H) -> i32;
    fn CreateCompatibleDC(dc: H) -> H;
    fn CreateCompatibleBitmap(dc: H, w: i32, h: i32) -> H;
    fn DeleteDC(dc: H) -> i32;
    fn BitBlt(dst: H, x: i32, y: i32, w: i32, h: i32, src: H, sx: i32, sy: i32, rop: u32) -> i32;
    fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> H;
    fn SelectClipRgn(dc: H, rgn: H) -> i32;
}

#[cfg_attr(all(windows, not(test)), link(name = "kernel32", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "kernel32"))]
unsafe extern "system" {
    fn GetTickCount64() -> u64;
    fn GetModuleHandleW(name: *const u16) -> H;
    fn GetModuleFileNameW(module: H, name: *mut u16, size: u32) -> u32;
    fn CreateWaitableTimerExW(attributes: *const c_void, name: *const u16, flags: u32, access: u32) -> H;
    fn SetWaitableTimer(timer: H, due: *const i64, period: i32,
                        completion: *const c_void, arg: *const c_void, resume: i32) -> i32;
    fn QueryPerformanceCounter(count: *mut i64) -> i32;
    fn QueryPerformanceFrequency(freq: *mut i64) -> i32;
    fn CreateFileW(name: *const u16, access: u32, share: u32, security: *const c_void,
                   disposition: u32, flags: u32, template: H) -> H;
    fn ReadFile(file: H, buffer: *mut u8, len: u32, read: *mut u32, overlapped: *mut c_void) -> i32;
    fn CreateThread(security: *const c_void, stack: usize, start: unsafe extern "system" fn(H) -> u32,
                    param: H, flags: u32, id: *mut u32) -> H;
    fn GetLastError() -> u32;
    fn CloseHandle(handle: H) -> i32;
    fn ExitProcess(code: u32) -> !;
}

#[cfg_attr(all(windows, not(test)), link(name = "winhttp", kind = "raw-dylib"))]
#[cfg_attr(all(windows, test), link(name = "winhttp"))]
unsafe extern "system" {
    fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> H;
    fn WinHttpConnect(session: H, server: *const u16, port: u16, reserved: u32) -> H;
    fn WinHttpSetTimeouts(handle: H, resolve: i32, connect: i32, send: i32, receive: i32) -> i32;
    fn WinHttpOpenRequest(connect: H, verb: *const u16, path: *const u16, version: *const u16,
                          referrer: *const u16, accept: *const *const u16, flags: u32) -> H;
    fn WinHttpSetOption(handle: H, option: u32, buffer: *const c_void, len: u32) -> i32;
    fn WinHttpSendRequest(request: H, headers: *const u16, headers_len: u32, body: *const c_void,
                          body_len: u32, total_len: u32, context: usize) -> i32;
    fn WinHttpReceiveResponse(request: H, reserved: *mut c_void) -> i32;
    fn WinHttpQueryHeaders(request: H, level: u32, name: *const u16, buffer: *mut c_void,
                           len: *mut u32, index: *mut u32) -> i32;
    fn WinHttpReadData(request: H, buffer: *mut c_void, len: u32, read: *mut u32) -> i32;
    fn WinHttpCloseHandle(handle: H) -> i32;
}

// The compiler may emit calls to these for struct copies and zeroing (it depends on the
// opt-level). ntdll is mapped into every process anyway, and only what's used gets imported.
#[cfg(all(windows, not(test)))]
#[link(name = "ntdll", kind = "raw-dylib")]
#[allow(dead_code)]
unsafe extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32;
}

const WM_DESTROY: u32 = 0x0002;
const WM_SIZE: u32 = 0x0005;
const WM_KILLFOCUS: u32 = 0x0008;
const WM_PAINT: u32 = 0x000F;
const WM_QUIT: u32 = 0x0012;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_APP_SHOT: u32 = 0x8000; // WM_APP: a model answer, wparam = approach, lparam = shot
const VK_ESCAPE: usize = 0x1B;
const VK_UP: usize = 0x26;
const VK_DOWN: usize = 0x28;
const VK_F11: usize = 0x7A;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const WS_MAXIMIZE: u32 = 0x0100_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const SW_MAXIMIZE: i32 = 3;
const PM_REMOVE: u32 = 1;
const QS_ALLINPUT: u32 = 0x04FF;
const INFINITE: u32 = 0xFFFF_FFFF;
const BLACK_BRUSH: i32 = 4;
const DC_BRUSH: i32 = 18;
const DC_PEN: i32 = 19;
const TRANSPARENT: i32 = 1;
const TA_CENTER: u32 = 6;
const TA_BASELINE: u32 = 24;
const SRCCOPY: u32 = 0x00CC_0020;
const NULLREGION: i32 = 1;
const GWL_STYLE: i32 = -16;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_FRAMECHANGED: u32 = 0x0020;
const SWP_NOOWNERZORDER: u32 = 0x0200;
const MONITOR_DEFAULTTONEAREST: u32 = 2;
const IDC_ARROW: usize = 32512;
const FW_BOLD: i32 = 700;
const DEFAULT_CHARSET: u32 = 1;
const CLEARTYPE_QUALITY: u32 = 5;
const CREATE_WAITABLE_TIMER_HIGH_RESOLUTION: u32 = 0x0000_0002;
const TIMER_ALL_ACCESS: u32 = 0x001F_0003;
const GENERIC_READ: u32 = 0x8000_0000;
const FILE_SHARE_READ: u32 = 1;
const OPEN_EXISTING: u32 = 3;
const INVALID_HANDLE: H = usize::MAX as H;
const STACK_SIZE_PARAM_IS_A_RESERVATION: u32 = 0x0001_0000;
const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: u32 = 4; // system/PAC proxy, like the rest of Windows
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
const WINHTTP_OPTION_ENABLE_FEATURE: u32 = 79;
const WINHTTP_ENABLE_SSL_REVOCATION: u32 = 1;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
const ERROR_WINHTTP_SECURE_CERT_REV_FAILED: u32 = 12057;

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

// ASCII string literal -> UTF-16 array, at compile time. `wz!` adds the terminating 0.
macro_rules! w {
    ($s:expr) => {{
        const S: &str = $s;
        const W: [u16; S.len()] = {
            let (b, mut out, mut i) = (S.as_bytes(), [0u16; S.len()], 0);
            while i < b.len() {
                out[i] = b[i] as u16;
                i += 1;
            }
            out
        };
        &W
    }};
}
macro_rules! wz {
    ($s:literal) => { w!(concat!($s, "\0")).as_ptr() };
}

// Shared mutable state. The game is only touched on the window thread; the model buffers are
// handed to the one worker thread in flight and not touched again until it answers.
struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}
impl<T> Global<T> {
    #[allow(clippy::mut_from_ref)]
    fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

// Fixed-size append-only buffer; anything past the end is dropped (callers check `full`).
struct Buf<T: Copy, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T: Copy + From<u8>, const N: usize> Buf<T, N> {
    const fn new(zero: T) -> Self {
        Buf { data: [zero; N], len: 0 }
    }
    fn push(&mut self, items: &[T]) {
        for &item in items {
            if self.len < N {
                self.data[self.len] = item;
                self.len += 1;
            }
        }
    }
    fn push_ascii(&mut self, s: &[u8]) {
        for &b in s {
            self.push(&[T::from(b)]);
        }
    }
    fn push_int(&mut self, n: i32) {
        let (mut digits, mut i, mut v) = ([0u8; 11], 11, n.unsigned_abs());
        loop {
            i -= 1;
            digits[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        if n < 0 {
            self.push_ascii(b"-");
        }
        self.push_ascii(&digits[i..]);
    }
    fn full(&self) -> bool {
        self.len >= N
    }
    #[cfg(test)]
    fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }
}

// core has no float rounding without libm, and these only need whole pixels.
fn floor(x: f64) -> f64 {
    let t = x as i64 as f64;
    if t > x { t - 1.0 } else { t }
}

fn ceil(x: f64) -> f64 {
    let t = x as i64 as f64;
    if t < x { t + 1.0 } else { t }
}

fn abs(x: f64) -> f64 {
    if x < 0.0 { -x } else { x }
}

// ---- Cloudflare clef-flash: picks which way to send the ball back (Engine.java's ATTACK) ----

const SHOT_NONE: u8 = 0; // no answer, an error, or "straight": keep the defensive spot
const SHOT_UP: u8 = 1;
const SHOT_DOWN: u8 = 2;

struct Api {
    session: H,
    connect: H,
    path: Buf<u16, 160>,
    headers: Buf<u16, 320>,
    body: Buf<u8, 1024>,
    skip_revocation: bool,
}

static API: Global<Api> = Global(UnsafeCell::new(Api {
    session: null_mut(),
    connect: null_mut(),
    path: Buf::new(0),
    headers: Buf::new(0),
    body: Buf::new(0),
    skip_revocation: false,
}));
static FILE: Global<[u8; 4096]> = Global(UnsafeCell::new([0; 4096]));
static REPLY: Global<[u8; 4096]> = Global(UnsafeCell::new([0; 4096]));

fn trim(mut s: &[u8]) -> &[u8] {
    while let [b' ' | b'\t' | b'\r', rest @ ..] = s {
        s = rest;
    }
    while let [rest @ .., b' ' | b'\t' | b'\r'] = s {
        s = rest;
    }
    s
}

// CLOUDFLARE_CLEF_API_KEY / CLOUDFLARE_ACCOUNT_ID from a .env file's text.
fn parse_env(text: &[u8]) -> Option<(&[u8], &[u8])> {
    let (mut key, mut account): (&[u8], &[u8]) = (&[], &[]);
    for line in text.split(|&b| b == b'\n') {
        if let Some(eq) = line.iter().position(|&b| b == b'=') {
            match trim(&line[..eq]) {
                b"CLOUDFLARE_CLEF_API_KEY" => key = trim(&line[eq + 1..]),
                b"CLOUDFLARE_ACCOUNT_ID" => account = trim(&line[eq + 1..]),
                _ => {}
            }
        }
    }
    let printable = |s: &[u8]| s.iter().all(|&b| (0x21..0x7F).contains(&b)); // no header injection
    (!key.is_empty() && !account.is_empty() && printable(key) && printable(account)).then_some((key, account))
}

// Read .env from the working directory, else next to the exe. No keys = defend only.
unsafe fn read_env() -> Option<&'static [u8]> {
    unsafe {
        let mut file = CreateFileW(wz!(".env"), GENERIC_READ, FILE_SHARE_READ, null(), OPEN_EXISTING, 0, null_mut());
        if file == INVALID_HANDLE {
            let mut path = [0u16; 300];
            let len = GetModuleFileNameW(null_mut(), path.as_mut_ptr(), 260) as usize;
            let slash = path[..len.min(260)].iter().rposition(|&c| c == b'\\' as u16)?;
            path[slash + 1..slash + 5].copy_from_slice(w!(".env"));
            path[slash + 5] = 0;
            file = CreateFileW(path.as_ptr(), GENERIC_READ, FILE_SHARE_READ, null(), OPEN_EXISTING, 0, null_mut());
            if file == INVALID_HANDLE {
                return None;
            }
        }
        let buf = FILE.get();
        let mut read = 0;
        let ok = ReadFile(file, buf.as_mut_ptr(), buf.len() as u32, &mut read, null_mut());
        CloseHandle(file);
        (ok != 0).then(|| &buf[..(read as usize).min(buf.len())])
    }
}

// One WinHTTP session and connection for the whole game: after the first request, later
// ones reuse the open TLS connection instead of a new process + handshake each time.
unsafe fn open_api() -> bool {
    let Some((key, account)) = (unsafe { read_env() }).and_then(parse_env) else { return false };
    let api = API.get();
    api.path.push_ascii(b"/client/v4/accounts/");
    api.path.push_ascii(account);
    api.path.push_ascii(b"/ai/run/@cf/cloudflare/clef-flash\0");
    api.headers.push_ascii(b"Authorization: Bearer ");
    api.headers.push_ascii(key);
    api.headers.push_ascii(b"\r\nContent-Type: application/json\0");
    if api.path.full() || api.headers.full() {
        return false;
    }
    unsafe {
        api.session = WinHttpOpen(wz!("Pongie"), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null(), null(), 0);
        if api.session.is_null() {
            return false;
        }
        WinHttpSetTimeouts(api.session, 10_000, 10_000, 10_000, 10_000);
        api.connect = WinHttpConnect(api.session, wz!("api.cloudflare.com"), 443, 0);
        !api.connect.is_null()
    }
}

fn build_body(body: &mut Buf<u8, 1024>, height: i32, hit_y: i32, player_middle: i32) {
    body.len = 0;
    body.push_ascii(br#"{"model":"clef-flash","state":"Pong. The screen is "#);
    body.push_int(height);
    body.push_ascii(br#" tall, y=0 is the top. You are the right paddle and you are about to hit the ball back at y="#);
    body.push_int(hit_y);
    body.push_ascii(br#". The other player's paddle is on the left side, centred at y="#);
    body.push_int(player_middle);
    body.push_ascii(concat!(
        r#". The ball bounces off the top and bottom walls.","questions":{"shot":{"#,
        r#""type":"choice","instructions":"Which way should you send the ball so it is "#,
        r#"hardest for the other player to reach? Aim away from where they are.","criteria":{"#,
        r#""up":"Send the ball upward, towards y=0","straight":"Send the ball straight across","#,
        r#""down":"Send the ball downward, towards the bottom"}}}}"#
    ).as_bytes());
}

// Pull "down" out of ..."choice":"down",... Keeps scanning like a regex would: the reply
// also contains "type":"choice" before the answer.
fn parse_choice(reply: &[u8]) -> u8 {
    const KEY: &[u8] = b"\"choice\"";
    let skip_space = |s: &[u8], mut i: usize| {
        while matches!(s.get(i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            i += 1;
        }
        i
    };
    for at in 0..reply.len() {
        if !reply[at..].starts_with(KEY) {
            continue;
        }
        let mut i = skip_space(reply, at + KEY.len());
        if reply.get(i) != Some(&b':') {
            continue;
        }
        i = skip_space(reply, i + 1);
        if reply.get(i) != Some(&b'"') {
            continue;
        }
        let start = i + 1;
        let word = reply[start..].iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'_').count();
        if word > 0 && reply.get(start + word) == Some(&b'"') {
            return match &reply[start..start + word] {
                b"up" => SHOT_UP,
                b"down" => SHOT_DOWN,
                _ => SHOT_NONE,
            };
        }
    }
    SHOT_NONE
}

unsafe fn post_request() -> H {
    let api = API.get();
    // Best-effort revocation, like curl's --ssl-revoke-best-effort: school proxies re-sign HTTPS
    // with a CA whose revocation list is unreachable, so retry once without the check. A revoked
    // certificate still fails.
    for _ in 0..2 {
        unsafe {
            let request = WinHttpOpenRequest(api.connect, wz!("POST"), api.path.data.as_ptr(), null(), null(),
                                             null(), WINHTTP_FLAG_SECURE);
            if request.is_null() {
                return null_mut();
            }
            if !api.skip_revocation {
                let on = WINHTTP_ENABLE_SSL_REVOCATION;
                WinHttpSetOption(request, WINHTTP_OPTION_ENABLE_FEATURE, (&raw const on).cast(), 4);
            }
            let len = api.body.len as u32;
            if WinHttpSendRequest(request, api.headers.data.as_ptr(), u32::MAX, api.body.data.as_ptr().cast(), len, len, 0) != 0
                && WinHttpReceiveResponse(request, null_mut()) != 0
            {
                return request;
            }
            let error = GetLastError();
            WinHttpCloseHandle(request);
            if error != ERROR_WINHTTP_SECURE_CERT_REV_FAILED || api.skip_revocation {
                return null_mut();
            }
            api.skip_revocation = true;
        }
    }
    null_mut()
}

// One request: the answer, or SHOT_NONE for no answer, an HTTP error or "straight".
fn fetch_shot() -> u8 {
    unsafe {
        let request = post_request();
        if request.is_null() {
            return SHOT_NONE;
        }
        let (mut status, mut size) = (0u32, 4u32);
        WinHttpQueryHeaders(request, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, null(),
                            (&raw mut status).cast(), &mut size, null_mut());
        let reply = REPLY.get();
        let mut len = 0;
        while len < reply.len() {
            let mut read = 0;
            let chunk = &mut reply[len..];
            if WinHttpReadData(request, chunk.as_mut_ptr().cast(), chunk.len() as u32, &mut read) == 0 || read == 0 {
                break;
            }
            len += read as usize;
        }
        WinHttpCloseHandle(request);
        if (200..300).contains(&status) { parse_choice(&reply[..len.min(reply.len())]) } else { SHOT_NONE }
    }
}

// Worker thread: one request, then post the answer back to the window and exit.
unsafe extern "system" fn ask_model(approach: H) -> u32 {
    let shot = fetch_shot();
    unsafe { PostMessageW(game().window, WM_APP_SHOT, approach as usize, shot as isize) };
    0
}

// ---- Game ----

#[derive(Clone, Copy, PartialEq)]
enum State {
    Countdown,
    Playing,
    GameOver,
}

#[derive(Clone, Copy, PartialEq)]
struct Paddle {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]
struct Ball {
    x: f64,
    y: f64,
    speed_x: f64,
    speed_y: f64,
}

struct Game {
    window: H,
    width: i32,
    height: i32,
    player: Paddle,
    opponent: Paddle,
    ball: Ball,
    opponent_target_y: f64,
    state: State,
    timer: i32,
    message_timer: i32,
    message: &'static [u16],
    moving_up: bool,
    moving_down: bool,
    fullscreen: bool,
    windowed_style: isize,
    font: H,
    back_dc: H,
    back_bitmap: H,
    original_bitmap: H,
    dirty_region: H,
    has_api: bool,
    approach: u32, // bumped every time the ball heads at the opponent
    asking: bool,  // a model request is in flight (one at a time, like Engine.java)
    landing_y: f64,
}

static GAME: Global<Game> = Global(UnsafeCell::new(Game::new()));

// Window calls like SetWindowPos send WM_SIZE straight back into window_proc, so never
// keep this reference across one of those calls.
fn game() -> &'static mut Game {
    GAME.get()
}

// Same order as Paddle.java: top first, then bottom (bottom wins on a tiny window).
fn keep_on_screen(y: f64, screen_height: i32) -> f64 {
    y.max(0.0).min((screen_height - PADDLE_HEIGHT) as f64)
}

fn overlaps(b: &Ball, p: &Paddle) -> bool {
    b.x < p.x + PADDLE_WIDTH as f64 && b.x + BALL_SIZE as f64 > p.x
        && b.y < p.y + PADDLE_HEIGHT as f64 && b.y + BALL_SIZE as f64 > p.y
}

impl Game {
    const fn new() -> Self {
        Game {
            window: null_mut(),
            width: 800,
            height: 600,
            player: Paddle { x: 20.0, y: 0.0 },
            opponent: Paddle { x: 765.0, y: 0.0 },
            ball: Ball { x: 0.0, y: 0.0, speed_x: START_SPEED, speed_y: START_SPEED },
            opponent_target_y: 300.0,
            state: State::Countdown,
            timer: 3 * FPS,
            message_timer: 0,
            message: w!("3"),
            moving_up: false,
            moving_down: false,
            fullscreen: false,
            windowed_style: 0,
            font: null_mut(),
            back_dc: null_mut(),
            back_bitmap: null_mut(),
            original_bitmap: null_mut(),
            dirty_region: null_mut(),
            has_api: false,
            approach: 0,
            asking: false,
            landing_y: 300.0,
        }
    }

    // Integer maths on purpose: matches Java's int positions exactly.
    fn reset(&mut self) {
        let paddle_y = (self.height / 2 - PADDLE_HEIGHT / 2) as f64;
        self.player = Paddle { x: 20.0, y: paddle_y };
        self.opponent = Paddle { x: (self.width - 35) as f64, y: paddle_y };
        self.ball = Ball {
            x: (self.width / 2 - BALL_SIZE / 2) as f64,
            y: (self.height / 2 - BALL_SIZE / 2) as f64,
            speed_x: START_SPEED,
            speed_y: START_SPEED,
        };
        if unsafe { GetTickCount64() } & 1 == 1 {
            self.ball.speed_x = -self.ball.speed_x;
        }
    }

    // Plan once per approach, like the Java engine thread, not every frame.
    // DEFEND: head for the landing spot now. ATTACK: ask the model (off-thread) which way to aim.
    fn aim(&mut self) {
        self.approach = self.approach.wrapping_add(1);
        if self.ball.speed_x <= 0.0 {
            self.opponent_target_y = (self.height / 2) as f64; // wait in the middle
            return;
        }
        self.landing_y = self.predict_landing_y();
        self.opponent_target_y = self.landing_y;
        if !self.has_api || self.asking {
            return;
        }
        let player_middle = self.player.y as i32 + PADDLE_HEIGHT / 2;
        build_body(&mut API.get().body, self.height, self.landing_y as i32, player_middle);
        // ponytail: a short-lived thread per approach (a few per rally); the connection is what's reused.
        let thread = unsafe {
            CreateThread(null(), 64 * 1024, ask_model, self.approach as usize as H,
                         STACK_SIZE_PARAM_IS_A_RESERVATION, null_mut())
        };
        if !thread.is_null() {
            self.asking = true;
            unsafe { CloseHandle(thread) };
        }
    }

    // Apply a model answer if it is for this approach and the ball is still far enough away.
    fn take_shot(&mut self, approach: u32, shot: u8) {
        self.asking = false;
        let frames_left = (self.opponent.x - (self.ball.x + BALL_SIZE as f64)) / self.ball.speed_x;
        if approach != self.approach || self.ball.speed_x <= 0.0 || frames_left <= 15.0 {
            return;
        }
        match shot {
            SHOT_UP => self.opponent_target_y = self.landing_y + AIM_SHIFT, // hit its top part
            SHOT_DOWN => self.opponent_target_y = self.landing_y - AIM_SHIFT, // hit its bottom part
            _ => {}
        }
    }

    fn predict_landing_y(&self) -> f64 {
        let (mut x, mut y, mut speed_y) = (self.ball.x, self.ball.y, self.ball.speed_y);
        while x + (BALL_SIZE as f64) < self.opponent.x {
            x += self.ball.speed_x;
            y += speed_y;
            if y <= 0.0 && speed_y < 0.0 {
                speed_y = -speed_y;
            }
            if y + BALL_SIZE as f64 >= self.height as f64 && speed_y > 0.0 {
                speed_y = -speed_y;
            }
        }
        (y as i32 + BALL_SIZE / 2) as f64 // Java truncates to int here
    }

    fn hit_paddle(&mut self, paddle: Paddle) {
        let ball_middle = self.ball.y + BALL_SIZE as f64 / 2.0;
        let paddle_middle = paddle.y + PADDLE_HEIGHT as f64 / 2.0;
        let hit_spot = ((ball_middle - paddle_middle) / (PADDLE_HEIGHT as f64 / 2.0)).clamp(-1.0, 1.0);
        self.ball.speed_x = -self.ball.speed_x * SPEED_UP;
        self.ball.speed_y = hit_spot * abs(self.ball.speed_x);
        self.aim();
    }

    fn invalidate_sprite(&self, x: f64, y: f64, w: i32, h: i32) {
        let area = Rect {
            left: floor(x) as i32 - 1,
            top: floor(y) as i32 - 1,
            right: ceil(x + w as f64) as i32 + 1,
            bottom: ceil(y + h as f64) as i32 + 1,
        };
        unsafe { InvalidateRect(self.window, &area, 0) };
    }

    // Old and new spot of anything that moved; a paddle standing still isn't redrawn.
    fn invalidate_paddle(&self, old: Paddle, new: Paddle) {
        if old != new {
            self.invalidate_sprite(old.x, old.y, PADDLE_WIDTH, PADDLE_HEIGHT);
            self.invalidate_sprite(new.x, new.y, PADDLE_WIDTH, PADDLE_HEIGHT);
        }
    }

    // Returns true when the whole window needs repainting (the message changed).
    fn update(&mut self) -> bool {
        if self.state == State::Playing {
            let (old_player, old_opponent, old_ball) = (self.player, self.opponent, self.ball);

            let speed = abs(self.ball.speed_x);
            let paddle_speed = (speed + 0.5) as i64 as f64; // Java paddles move whole pixels (round)
            if self.moving_up {
                self.player.y -= paddle_speed;
            }
            if self.moving_down {
                self.player.y += paddle_speed;
            }
            self.player.y = keep_on_screen(self.player.y, self.height);

            let opponent_middle = self.opponent.y + PADDLE_HEIGHT as f64 / 2.0;
            if opponent_middle > self.opponent_target_y + speed {
                self.opponent.y -= paddle_speed;
            }
            if opponent_middle < self.opponent_target_y - speed {
                self.opponent.y += paddle_speed;
            }
            self.opponent.y = keep_on_screen(self.opponent.y, self.height);

            self.ball.x += self.ball.speed_x;
            self.ball.y += self.ball.speed_y;
            if self.ball.y <= 0.0 && self.ball.speed_y < 0.0 {
                self.ball.speed_y = -self.ball.speed_y;
            }
            if self.ball.y + BALL_SIZE as f64 >= self.height as f64 && self.ball.speed_y > 0.0 {
                self.ball.speed_y = -self.ball.speed_y;
            }

            if overlaps(&self.ball, &self.player) && self.ball.speed_x < 0.0 && self.ball.x > self.player.x {
                self.hit_paddle(self.player);
            }
            if overlaps(&self.ball, &self.opponent) && self.ball.speed_x > 0.0 && self.ball.x < self.opponent.x {
                self.hit_paddle(self.opponent);
            }

            if self.ball.x <= 0.0 || self.ball.x + BALL_SIZE as f64 >= self.width as f64 {
                self.reset();
                self.state = State::GameOver;
                self.message = w!("GAME OVER");
                self.timer = 3 * FPS;
                return true;
            }
            self.invalidate_paddle(old_player, self.player);
            self.invalidate_paddle(old_opponent, self.opponent);
            self.invalidate_sprite(old_ball.x, old_ball.y, BALL_SIZE, BALL_SIZE);
            self.invalidate_sprite(self.ball.x, self.ball.y, BALL_SIZE, BALL_SIZE);
            return self.tick_message();
        }

        self.timer -= 1;
        if self.state == State::GameOver {
            if self.timer == 0 {
                self.state = State::Countdown;
                self.message = w!("3");
                self.timer = 3 * FPS;
                return true;
            }
            return false;
        }

        match self.timer {
            t if t == 2 * FPS => self.message = w!("2"),
            t if t == FPS => self.message = w!("1"),
            0 => {
                self.state = State::Playing;
                self.message = w!("GO!");
                self.message_timer = FPS;
                self.aim();
            }
            _ => return self.tick_message(),
        }
        true
    }

    // Hides "GO!" after a second.
    fn tick_message(&mut self) -> bool {
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = &[];
                return true;
            }
        }
        false
    }

    fn paint(&self, dc: H) {
        let area = Rect { left: 0, top: 0, right: self.width, bottom: self.height };
        // ponytail: GAME OVER overlay (black at alpha 180) pre-multiplied into the
        // sprite colours; the background is already black so no AlphaBlend needed.
        let keep = if self.state == State::GameOver { 255 - 180 } else { 255 };
        let shade = |r: u32, g: u32, b: u32| rgb(r * keep / 255, g * keep / 255, b * keep / 255);
        unsafe {
            FillRect(dc, &area, GetStockObject(BLACK_BRUSH));

            let brush = GetStockObject(DC_BRUSH);
            SetDCBrushColor(dc, shade(0, 0, 255)); // Java's Color.BLUE
            for p in [self.player, self.opponent] {
                let (x, y) = (p.x as i32, p.y as i32);
                let r = Rect { left: x, top: y, right: x + PADDLE_WIDTH, bottom: y + PADDLE_HEIGHT };
                FillRect(dc, &r, brush);
            }

            SetDCBrushColor(dc, shade(255, 0, 0));
            SetDCPenColor(dc, shade(255, 0, 0));
            let old_brush = SelectObject(dc, brush);
            let old_pen = SelectObject(dc, GetStockObject(DC_PEN));
            let (x, y) = (self.ball.x as i32, self.ball.y as i32);
            Ellipse(dc, x, y, x + BALL_SIZE, y + BALL_SIZE);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);

            if !self.message.is_empty() {
                let old_font = SelectObject(dc, self.font);
                SetTextColor(dc, rgb(255, 255, 255));
                SetBkMode(dc, TRANSPARENT);
                SetTextAlign(dc, TA_CENTER | TA_BASELINE); // Java: centred, baseline at mid-height
                TextOutW(dc, self.width / 2, self.height / 2, self.message.as_ptr(), self.message.len() as i32);
                SelectObject(dc, old_font);
            }
        }
    }

    fn release_back_buffer(&mut self) {
        if self.back_dc.is_null() {
            return;
        }
        unsafe {
            if !self.original_bitmap.is_null() {
                SelectObject(self.back_dc, self.original_bitmap);
            }
            if !self.back_bitmap.is_null() {
                DeleteObject(self.back_bitmap);
            }
            DeleteDC(self.back_dc);
        }
        self.back_dc = null_mut();
        self.back_bitmap = null_mut();
        self.original_bitmap = null_mut();
    }

    fn create_back_buffer(&mut self, window: H) {
        self.release_back_buffer();
        unsafe {
            let screen = GetDC(window);
            self.back_dc = CreateCompatibleDC(screen);
            self.back_bitmap = CreateCompatibleBitmap(screen, self.width, self.height);
            ReleaseDC(window, screen);
            if self.back_dc.is_null() || self.back_bitmap.is_null() {
                self.release_back_buffer();
                return;
            }
            self.original_bitmap = SelectObject(self.back_dc, self.back_bitmap);
        }
    }
}

fn toggle_fullscreen() {
    let (window, fullscreen) = (game().window, game().fullscreen);
    unsafe {
        let mut monitor: MonitorInfo = core::mem::zeroed();
        monitor.size = size_of::<MonitorInfo>() as u32;
        GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &mut monitor);
        if !fullscreen {
            let style = GetWindowLongPtrW(window, GWL_STYLE);
            game().windowed_style = style;
            game().fullscreen = true;
            let m = monitor.monitor;
            SetWindowLongPtrW(window, GWL_STYLE, style & !(WS_OVERLAPPEDWINDOW as isize));
            SetWindowPos(window, null_mut(), m.left, m.top, m.right - m.left, m.bottom - m.top,
                         SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        } else {
            // Like Java's pack(): a normal (not maximized) window with an 800x600 game area, centred.
            game().fullscreen = false;
            let style = game().windowed_style & !(WS_MAXIMIZE as isize);
            let mut r = Rect { left: 0, top: 0, right: 800, bottom: 600 };
            AdjustWindowRectEx(&mut r, style as u32, 0, 0);
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            let work = monitor.work;
            SetWindowLongPtrW(window, GWL_STYLE, style);
            SetWindowPos(window, null_mut(), work.left + (work.right - work.left - w) / 2,
                         work.top + (work.bottom - work.top - h) / 2, w, h,
                         SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        }
    }
}

unsafe extern "system" fn window_proc(window: H, message: u32, wparam: usize, lparam: isize) -> isize {
    let up = wparam == 'E' as usize || wparam == VK_UP;
    let down = wparam == 'D' as usize || wparam == VK_DOWN;
    unsafe {
        match message {
            WM_SIZE => {
                let g = game();
                g.width = (lparam & 0xFFFF).max(1) as i32;
                g.height = ((lparam >> 16) & 0xFFFF).max(1) as i32;
                g.create_back_buffer(window);
                g.opponent.x = (g.width - 35) as f64; // keep it on the right edge
                if g.state != State::Playing {
                    g.reset();
                }
                InvalidateRect(window, null(), 0);
                0
            }
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                let g = game();
                let region_type = GetUpdateRgn(window, g.dirty_region, 0);
                let mut ps: PaintStruct = core::mem::zeroed();
                let screen = BeginPaint(window, &mut ps);
                if !g.back_dc.is_null() {
                    if region_type > NULLREGION {
                        SelectClipRgn(g.back_dc, g.dirty_region);
                    }
                    g.paint(g.back_dc);
                    let d = ps.paint;
                    BitBlt(screen, d.left, d.top, d.right - d.left, d.bottom - d.top,
                           g.back_dc, d.left, d.top, SRCCOPY);
                    SelectClipRgn(g.back_dc, null_mut());
                }
                EndPaint(window, &ps);
                0
            }
            WM_APP_SHOT => {
                game().take_shot(wparam as u32, lparam as u8);
                0
            }
            WM_KEYDOWN => {
                if lparam & (1 << 30) == 0 && (wparam == VK_F11 || (wparam == VK_ESCAPE && game().fullscreen)) {
                    toggle_fullscreen();
                }
                let g = game();
                g.moving_up |= up;
                g.moving_down |= down;
                0
            }
            WM_KEYUP => {
                let g = game();
                g.moving_up &= !up;
                g.moving_down &= !down;
                0
            }
            WM_KILLFOCUS => {
                let g = game();
                g.moving_up = false;
                g.moving_down = false;
                0
            }
            WM_DESTROY => {
                game().release_back_buffer();
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

unsafe fn run() {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class_name = wz!("PongRsWindow");
        let class = WndClass {
            style: 0,
            wnd_proc: window_proc,
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: null_mut(),
            cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
            background: GetStockObject(BLACK_BRUSH),
            menu_name: null(),
            class_name,
        };
        if RegisterClassW(&class) == 0 {
            return;
        }

        let g = game();
        // Negative height = em size, which is what Java's 72pt Arial means.
        g.font = CreateFontW(-72, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0,
                             CLEARTYPE_QUALITY, 0, wz!("Arial"));
        g.dirty_region = CreateRectRgn(0, 0, 0, 0);
        g.has_api = open_api();
        g.reset();
        let window = CreateWindowExW(0, class_name, wz!("Pongie (Rust)"), WS_OVERLAPPEDWINDOW,
                                     CW_USEDEFAULT, CW_USEDEFAULT, 800, 600,
                                     null_mut(), null_mut(), instance, null_mut());
        if window.is_null() {
            return;
        }
        game().window = window;
        ShowWindow(window, SW_MAXIMIZE);
        UpdateWindow(window);

        // Sub-millisecond waits on Windows 10 1803+; plain ~15ms sleeps otherwise.
        let timer = CreateWaitableTimerExW(null(), null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS);
        let (mut freq, mut now) = (0i64, 0i64);
        QueryPerformanceFrequency(&mut freq);
        let frame = freq / FPS as i64;
        QueryPerformanceCounter(&mut now);
        let mut next_frame = now;
        let mut msg: Msg = core::mem::zeroed();
        'running: loop {
            QueryPerformanceCounter(&mut now);
            if now < next_frame {
                let wait_100ns = (next_frame - now) * 10_000_000 / freq;
                if !timer.is_null() {
                    let due = -wait_100ns; // negative = relative
                    SetWaitableTimer(timer, &due, 0, null(), null(), 0);
                    MsgWaitForMultipleObjects(1, &timer, 0, INFINITE, QS_ALLINPUT);
                } else {
                    MsgWaitForMultipleObjects(0, null(), 0, (wait_100ns / 10_000).max(1) as u32, QS_ALLINPUT);
                }
            }

            // No TranslateMessage: nothing here reads WM_CHAR.
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'running;
                }
                DispatchMessageW(&msg);
            }

            QueryPerformanceCounter(&mut now);
            if now >= next_frame {
                if game().update() {
                    InvalidateRect(window, null(), 0);
                }
                next_frame += frame;
                if next_frame <= now {
                    next_frame = now + frame; // fell behind: don't burst to catch up
                }
            }
        }
        // ExitProcess frees the font, region, timer and WinHTTP handles.
    }
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "system" fn mainCRTStartup() -> ! {
    unsafe {
        run();
        ExitProcess(0)
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { ExitProcess(1) }
}

// The linker wants this marker whenever floating point is used without a C runtime.
#[cfg(not(test))]
#[unsafe(no_mangle)]
static _fltused: i32 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opponent_predicts_bounces_and_paddles_aim() {
        let mut g = Game::new();
        g.opponent.x = 500.0;

        g.ball = Ball { x: 0.0, y: 100.0, speed_x: 10.0, speed_y: 0.0 };
        assert_eq!(g.predict_landing_y(), 115.0);

        // Up 5 steps to the top wall, bounce, then down 42 more: y = 420.
        g.ball = Ball { x: 0.0, y: 50.0, speed_x: 10.0, speed_y: -10.0 };
        assert_eq!(g.predict_landing_y(), 435.0);

        let paddle = Paddle { x: 20.0, y: 100.0 };
        g.ball = Ball { x: 30.0, y: 135.0, speed_x: -10.0, speed_y: 3.0 }; // dead centre
        g.hit_paddle(paddle);
        assert!((g.ball.speed_x - 10.5).abs() < 1e-9);
        assert_eq!(g.ball.speed_y, 0.0);

        g.ball = Ball { x: 30.0, y: 185.0, speed_x: -10.0, speed_y: 0.0 }; // bottom edge
        g.hit_paddle(paddle);
        assert!((g.ball.speed_y - 10.5).abs() < 1e-9);

        // A model answer for this approach shifts the aim; a stale one is ignored.
        g.ball = Ball { x: 0.0, y: 100.0, speed_x: 10.0, speed_y: 0.0 };
        g.aim();
        g.take_shot(g.approach - 1, SHOT_UP);
        assert_eq!(g.opponent_target_y, 115.0);
        g.take_shot(g.approach, SHOT_DOWN);
        assert_eq!(g.opponent_target_y, 115.0 - AIM_SHIFT);
    }

    #[test]
    fn reads_env_and_the_models_choice_and_clamps_like_java() {
        assert_eq!(parse_choice(br#"{"result":{"choice" : "down","confidence":0.4}}"#), SHOT_DOWN);
        assert_eq!(parse_choice(br#"{"errors":[]}"#), SHOT_NONE);
        let real = br#"{"result":{"model":"clef-flash","answers":{"shot":{"type":"choice","choice":"up","probabilities":{"up":0.27}}}}}"#;
        assert_eq!(parse_choice(real), SHOT_UP);
        assert_eq!(parse_choice(br#""choice":"straight""#), SHOT_NONE);
        assert_eq!(keep_on_screen(-5.0, 600), 0.0);
        assert_eq!(keep_on_screen(550.0, 600), 500.0);
        assert_eq!((floor(-1.5), ceil(-1.5), floor(2.0), ceil(2.25)), (-2.0, -1.0, 2.0, 3.0));

        let env = b"# keys\r\nCLOUDFLARE_ACCOUNT_ID = abc123\r\nCLOUDFLARE_CLEF_API_KEY=k-ey_9\r\n";
        assert_eq!(parse_env(env), Some((&b"k-ey_9"[..], &b"abc123"[..])));
        assert_eq!(parse_env(b"CLOUDFLARE_CLEF_API_KEY=x\nCLOUDFLARE_ACCOUNT_ID=a b"), None);

        let mut body = Buf::<u8, 1024>::new(0);
        build_body(&mut body, 600, -450, 100);
        let text = core::str::from_utf8(body.as_slice()).unwrap();
        assert!(!body.full() && text.contains("is 600 tall") && text.contains("y=-450.") && text.ends_with("}}}}"));
        let json_braces = text.matches('{').count() == text.matches('}').count();
        assert!(json_braces, "{text}");
    }

    // Real network call with ../pong/.env: cargo test --release -- --ignored
    #[test]
    #[ignore]
    fn live_model_answers() {
        std::env::set_current_dir("../pong").unwrap();
        assert!(unsafe { open_api() }, "no keys in ../pong/.env");
        for _ in 0..2 { // the second request reuses the connection
            build_body(&mut API.get().body, 600, 450, 100);
            REPLY.get().fill(0);
            let started = std::time::Instant::now();
            let shot = fetch_shot();
            println!("shot {shot} in {:?}", started.elapsed());
            let raw = String::from_utf8_lossy(&REPLY.get()[..]).trim_end_matches('\0').to_string();
            assert!(raw.contains("\"choice\""), "no answer from clef-flash: {raw}");
        }
    }
}
