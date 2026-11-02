#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::ffi::c_void;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

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

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const WndClass) -> u16;
    fn CreateWindowExW(ex_style: u32, class: *const u16, title: *const u16, style: u32,
                       x: i32, y: i32, w: i32, h: i32,
                       parent: H, menu: H, instance: H, param: H) -> H;
    fn DefWindowProcW(hwnd: H, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn ShowWindow(hwnd: H, cmd: i32) -> i32;
    fn UpdateWindow(hwnd: H) -> i32;
    fn PeekMessageW(msg: *mut Msg, hwnd: H, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
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

#[link(name = "gdi32")]
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

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTickCount64() -> u64;
    fn GetModuleHandleW(name: *const u16) -> H;
    fn CreateWaitableTimerExW(attributes: *const c_void, name: *const u16, flags: u32, access: u32) -> H;
    fn SetWaitableTimer(timer: H, due: *const i64, period: i32,
                        completion: *const c_void, arg: *const c_void, resume: i32) -> i32;
    fn CloseHandle(handle: H) -> i32;
}

const WM_DESTROY: u32 = 0x0002;
const WM_SIZE: u32 = 0x0005;
const WM_KILLFOCUS: u32 = 0x0008;
const WM_PAINT: u32 = 0x000F;
const WM_QUIT: u32 = 0x0012;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
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
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const TIMER_ALL_ACCESS: u32 = 0x001F_0003;

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

// ---- Cloudflare clef-flash: picks which way to send the ball back (Engine.java's ATTACK) ----

struct Api {
    key: String,
    account: String,
}

// Read CLOUDFLARE_CLEF_API_KEY / CLOUDFLARE_ACCOUNT_ID from .env in the working
// directory, else next to the exe. No keys = defend only, like the Java engine.
fn load_api() -> Option<Api> {
    let beside_exe = std::env::current_exe().ok()?.with_file_name(".env");
    let text = std::fs::read_to_string(".env").or_else(|_| std::fs::read_to_string(beside_exe)).ok()?;
    let (mut key, mut account) = (String::new(), String::new());
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "CLOUDFLARE_CLEF_API_KEY" => key = v.trim().to_string(),
                "CLOUDFLARE_ACCOUNT_ID" => account = v.trim().to_string(),
                _ => {}
            }
        }
    }
    (!key.is_empty() && !account.is_empty()).then_some(Api { key, account })
}

// Pull "down" out of ..."choice":"down",... (same job as Engine.java's regex). Keeps
// scanning like a regex would: the reply also contains "type":"choice" before the answer.
fn parse_choice(reply: &str) -> Option<String> {
    reply.match_indices("\"choice\"").find_map(|(at, key)| {
        let rest = reply[at + key.len()..].trim_start().strip_prefix(':')?;
        let rest = rest.trim_start().strip_prefix('"')?;
        let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
        (end > 0 && rest[end..].starts_with('"')).then(|| rest[..end].to_string())
    })
}

// ponytail: Windows' own curl.exe (Windows 10 1803+) instead of an HTTP crate. It uses the
// Windows certificate store like the Java engine does, and the key goes in on stdin so it
// never shows up in the process list.
fn ask_model(api: &Api, height: i32, hit_y: i32, player_middle: i32) -> Option<String> {
    let body = format!(
        concat!(
            r#"{{"model":"clef-flash","state":"Pong. The screen is {} tall, y=0 is the top. "#,
            r#"You are the right paddle and you are about to hit the ball back at y={}. "#,
            r#"The other player's paddle is on the left side, centred at y={}. "#,
            r#"The ball bounces off the top and bottom walls.","questions":{{"shot":{{"#,
            r#""type":"choice","instructions":"Which way should you send the ball so it is "#,
            r#"hardest for the other player to reach? Aim away from where they are.","criteria":{{"#,
            r#""up":"Send the ball upward, towards y=0","straight":"Send the ball straight across","#,
            r#""down":"Send the ball downward, towards the bottom"}}}}}}}}"#
        ),
        height, hit_y, player_middle
    );
    let quote = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let config = format!(
        "url = \"https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/cloudflare/clef-flash\"\n\
         header = \"Authorization: Bearer {}\"\n\
         header = \"Content-Type: application/json\"\n\
         data-binary = \"{}\"\n",
        quote(&api.account), quote(&api.key), quote(&body)
    );
    let mut curl = Command::new("curl.exe")
        // best-effort revocation: school proxies re-sign HTTPS with a CA whose revocation list is
        // unreachable. Java skips revocation checks entirely; this still rejects revoked certs.
        .args(["--silent", "--fail", "--ssl-revoke-best-effort", "--max-time", "10", "--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;
    curl.stdin.take()?.write_all(config.as_bytes()).ok()?;
    let out = curl.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_choice(&String::from_utf8_lossy(&out.stdout))
}

// ---- Game ----

#[derive(Clone, Copy, PartialEq)]
enum State {
    Countdown,
    Playing,
    GameOver,
}

#[derive(Clone, Copy)]
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
    message: &'static str,
    moving_up: bool,
    moving_down: bool,
    fullscreen: bool,
    windowed_style: isize,
    font: H,
    back_dc: H,
    back_bitmap: H,
    original_bitmap: H,
    api: Option<Arc<Api>>,
    approach: u32, // bumped every time the ball heads at the opponent
    asking: bool,  // a model request is in flight (one at a time, like Engine.java)
    landing_y: f64,
    shots: (Sender<(u32, Option<String>)>, Receiver<(u32, Option<String>)>),
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
    fn new() -> Self {
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
            message: "3",
            moving_up: false,
            moving_down: false,
            fullscreen: false,
            windowed_style: 0,
            font: null_mut(),
            back_dc: null_mut(),
            back_bitmap: null_mut(),
            original_bitmap: null_mut(),
            api: None,
            approach: 0,
            asking: false,
            landing_y: 300.0,
            shots: channel(),
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
        self.approach += 1;
        if self.ball.speed_x <= 0.0 {
            self.opponent_target_y = (self.height / 2) as f64; // wait in the middle
            return;
        }
        self.landing_y = self.predict_landing_y();
        self.opponent_target_y = self.landing_y;
        let Some(api) = self.api.clone() else { return };
        if self.asking {
            return;
        }
        self.asking = true;
        let (approach, height, hit_y) = (self.approach, self.height, self.landing_y as i32);
        let player_middle = self.player.y as i32 + PADDLE_HEIGHT / 2;
        let reply = self.shots.0.clone();
        std::thread::spawn(move || {
            let _ = reply.send((approach, ask_model(&api, height, hit_y, player_middle)));
        });
    }

    // Apply a model answer if it is for this approach and the ball is still far enough away.
    fn take_shot(&mut self) {
        while let Ok((approach, shot)) = self.shots.1.try_recv() {
            self.asking = false;
            let frames_left = (self.opponent.x - (self.ball.x + BALL_SIZE as f64)) / self.ball.speed_x;
            if approach != self.approach || self.ball.speed_x <= 0.0 || frames_left <= 15.0 {
                continue;
            }
            match shot.as_deref() {
                Some("up") => self.opponent_target_y = self.landing_y + AIM_SHIFT, // hit its top part
                Some("down") => self.opponent_target_y = self.landing_y - AIM_SHIFT, // hit its bottom part
                _ => {}
            }
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
        self.ball.speed_y = hit_spot * self.ball.speed_x.abs();
        self.aim();
    }

    fn invalidate_sprite(&self, x: f64, y: f64, w: i32, h: i32) {
        let area = Rect {
            left: x.floor() as i32 - 1,
            top: y.floor() as i32 - 1,
            right: (x + w as f64).ceil() as i32 + 1,
            bottom: (y + h as f64).ceil() as i32 + 1,
        };
        unsafe { InvalidateRect(self.window, &area, 0) };
    }

    fn invalidate_sprites(&self) {
        self.invalidate_sprite(self.player.x, self.player.y, PADDLE_WIDTH, PADDLE_HEIGHT);
        self.invalidate_sprite(self.opponent.x, self.opponent.y, PADDLE_WIDTH, PADDLE_HEIGHT);
        self.invalidate_sprite(self.ball.x, self.ball.y, BALL_SIZE, BALL_SIZE);
    }

    // Returns true when the whole window needs repainting (the message changed).
    fn update(&mut self) -> bool {
        if self.state == State::Playing {
            self.invalidate_sprites();
            self.take_shot();

            let speed = self.ball.speed_x.abs();
            let paddle_speed = speed.round(); // Java paddles move whole pixels
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
                self.message = "GAME OVER";
                self.timer = 3 * FPS;
                return true;
            }
            self.invalidate_sprites();
            return self.tick_message();
        }

        self.timer -= 1;
        if self.state == State::GameOver {
            if self.timer == 0 {
                self.state = State::Countdown;
                self.message = "3";
                self.timer = 3 * FPS;
                return true;
            }
            return false;
        }

        match self.timer {
            t if t == 2 * FPS => self.message = "2",
            t if t == FPS => self.message = "1",
            0 => {
                self.state = State::Playing;
                self.message = "GO!";
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
                self.message = "";
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
                // Messages are short and fixed, so encode on the stack instead of allocating.
                let mut text = [0u16; 16];
                let mut len = 0;
                for (slot, unit) in text.iter_mut().zip(self.message.encode_utf16()) {
                    *slot = unit;
                    len += 1;
                }
                let old_font = SelectObject(dc, self.font);
                SetTextColor(dc, rgb(255, 255, 255));
                SetBkMode(dc, TRANSPARENT);
                SetTextAlign(dc, TA_CENTER | TA_BASELINE); // Java: centred, baseline at mid-height
                TextOutW(dc, self.width / 2, self.height / 2, text.as_ptr(), len);
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

thread_local! {
    static GAME: RefCell<Game> = RefCell::new(Game::new());
}

// Window calls like SetWindowPos send WM_SIZE straight back into window_proc,
// so never make one while holding this borrow (RefCell would panic, not corrupt).
fn with<R>(f: impl FnOnce(&mut Game) -> R) -> R {
    GAME.with_borrow_mut(f)
}

fn toggle_fullscreen() {
    let (window, fullscreen) = with(|g| (g.window, g.fullscreen));
    unsafe {
        let mut monitor: MonitorInfo = std::mem::zeroed();
        monitor.size = size_of::<MonitorInfo>() as u32;
        GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &mut monitor);
        if !fullscreen {
            let style = GetWindowLongPtrW(window, GWL_STYLE);
            with(|g| {
                g.windowed_style = style;
                g.fullscreen = true;
            });
            let m = monitor.monitor;
            SetWindowLongPtrW(window, GWL_STYLE, style & !(WS_OVERLAPPEDWINDOW as isize));
            SetWindowPos(window, null_mut(), m.left, m.top, m.right - m.left, m.bottom - m.top,
                         SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        } else {
            // Like Java's pack(): a normal (not maximized) window with an 800x600 game area, centred.
            let style = with(|g| {
                g.fullscreen = false;
                g.windowed_style & !(WS_MAXIMIZE as isize)
            });
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
                with(|g| {
                    g.width = (lparam & 0xFFFF).max(1) as i32;
                    g.height = ((lparam >> 16) & 0xFFFF).max(1) as i32;
                    g.create_back_buffer(window);
                    g.opponent.x = (g.width - 35) as f64; // keep it on the right edge
                    if g.state != State::Playing {
                        g.reset();
                    }
                });
                InvalidateRect(window, null(), 0);
                0
            }
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                let dirty_region = CreateRectRgn(0, 0, 0, 0);
                let region_type = GetUpdateRgn(window, dirty_region, 0);
                let mut ps: PaintStruct = std::mem::zeroed();
                let screen = BeginPaint(window, &mut ps);
                with(|g| {
                    if g.back_dc.is_null() {
                        return;
                    }
                    if region_type > NULLREGION {
                        SelectClipRgn(g.back_dc, dirty_region);
                    }
                    g.paint(g.back_dc);
                    let d = ps.paint;
                    BitBlt(screen, d.left, d.top, d.right - d.left, d.bottom - d.top,
                           g.back_dc, d.left, d.top, SRCCOPY);
                    SelectClipRgn(g.back_dc, null_mut());
                });
                EndPaint(window, &ps);
                DeleteObject(dirty_region);
                0
            }
            WM_KEYDOWN => {
                if lparam & (1 << 30) == 0 {
                    let fullscreen = with(|g| g.fullscreen);
                    if wparam == VK_F11 || (wparam == VK_ESCAPE && fullscreen) {
                        toggle_fullscreen();
                    }
                }
                with(|g| {
                    g.moving_up |= up;
                    g.moving_down |= down;
                });
                0
            }
            WM_KEYUP => {
                with(|g| {
                    g.moving_up &= !up;
                    g.moving_down &= !down;
                });
                0
            }
            WM_KILLFOCUS => {
                with(|g| {
                    g.moving_up = false;
                    g.moving_down = false;
                });
                0
            }
            WM_DESTROY => {
                with(|g| g.release_back_buffer());
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

fn main() {
    let class_name = wide("PongRsWindow");
    let title = wide("Pongie (Rust)");
    let face = wide("Arial");
    unsafe {
        let instance = GetModuleHandleW(null());
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
            class_name: class_name.as_ptr(),
        };
        if RegisterClassW(&class) == 0 {
            return;
        }

        // Negative height = em size, which is what Java's 72pt Arial means.
        let font = CreateFontW(-72, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0,
                               CLEARTYPE_QUALITY, 0, face.as_ptr());
        let window = CreateWindowExW(0, class_name.as_ptr(), title.as_ptr(), WS_OVERLAPPEDWINDOW,
                                     CW_USEDEFAULT, CW_USEDEFAULT, 800, 600,
                                     null_mut(), null_mut(), instance, null_mut());
        if window.is_null() {
            DeleteObject(font);
            return;
        }
        with(|g| {
            g.window = window;
            g.font = font;
            g.api = load_api().map(Arc::new);
            g.reset();
        });
        ShowWindow(window, SW_MAXIMIZE);
        UpdateWindow(window);

        // Sub-millisecond waits on Windows 10 1803+; plain ~15ms sleeps otherwise.
        let timer = CreateWaitableTimerExW(null(), null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                                           TIMER_ALL_ACCESS);
        let frame = Duration::from_nanos(1_000_000_000 / FPS as u64);
        let mut next_frame = Instant::now();
        let mut msg: Msg = std::mem::zeroed();
        'running: loop {
            let now = Instant::now();
            if now < next_frame {
                let wait = next_frame - now;
                if !timer.is_null() {
                    let due = -((wait.as_nanos() / 100) as i64); // negative = relative, 100ns units
                    SetWaitableTimer(timer, &due, 0, null(), null(), 0);
                    MsgWaitForMultipleObjects(1, &timer, 0, INFINITE, QS_ALLINPUT);
                } else {
                    MsgWaitForMultipleObjects(0, null(), 0, wait.as_millis().max(1) as u32, QS_ALLINPUT);
                }
            }

            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'running;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let now = Instant::now();
            if now >= next_frame {
                if with(|g| g.update()) {
                    InvalidateRect(window, null(), 0);
                }
                next_frame += frame;
                if next_frame <= now {
                    next_frame = now + frame; // fell behind: don't burst to catch up
                }
            }
        }

        if !timer.is_null() {
            CloseHandle(timer);
        }
        DeleteObject(font);
    }
}

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
        g.shots.0.send((g.approach - 1, Some("up".into()))).unwrap();
        g.take_shot();
        assert_eq!(g.opponent_target_y, 115.0);
        g.shots.0.send((g.approach, Some("down".into()))).unwrap();
        g.take_shot();
        assert_eq!(g.opponent_target_y, 115.0 - AIM_SHIFT);
    }

    #[test]
    fn reads_the_models_choice_and_clamps_like_java() {
        assert_eq!(parse_choice(r#"{"result":{"choice" : "down","confidence":0.4}}"#).as_deref(), Some("down"));
        assert_eq!(parse_choice(r#"{"errors":[]}"#), None);
        let real = r#"{"result":{"model":"clef-flash","answers":{"shot":{"type":"choice","choice":"down","probabilities":{"up":0.27}}}}}"#;
        assert_eq!(parse_choice(real).as_deref(), Some("down"));
        assert_eq!(keep_on_screen(-5.0, 600), 0.0);
        assert_eq!(keep_on_screen(550.0, 600), 500.0);
    }

    // Real network call with ../pong/.env: cargo test --release -- --ignored
    #[test]
    #[ignore]
    fn live_model_answers() {
        std::env::set_current_dir("../pong").unwrap();
        let api = load_api().expect("no keys in ../pong/.env");
        let shot = ask_model(&api, 600, 450, 100).expect("no answer from clef-flash");
        assert!(["up", "straight", "down"].contains(&shot.as_str()), "{shot}");
    }
}
