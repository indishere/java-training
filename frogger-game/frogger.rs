use minifb::{Key, Window, WindowOptions};

const WINDOW_WIDTH: usize = 400;
const WINDOW_HEIGHT: usize = 300;

const WHITE: u32 = 0xFFFFFF;
const YELLOW: u32 = 0xFFFF00;
const BLACK: u32 = 0x000000;
const RED: u32 = 0xFF0000;

fn set_pixel(buffer: &mut [u32], x: i32, y: i32, pixel_color: u32) {
    if x >= 0 && x < WINDOW_WIDTH as i32 && y >= 0 && y < WINDOW_HEIGHT as i32 {
        let index = y as usize * WINDOW_WIDTH + x as usize;
        buffer[index] = pixel_color;
    }
}

// This replaces Java's graphics.fillOval for a circle.
fn draw_filled_circle(buffer: &mut [u32], center_x: i32, center_y: i32, radius: i32, fill_color: u32) {
    for y in center_y - radius..=center_y + radius {
        for x in center_x - radius..=center_x + radius {
            let distance_squared = (x - center_x) * (x - center_x) + (y - center_y) * (y - center_y);
            if distance_squared <= radius * radius {
                set_pixel(buffer, x, y, fill_color);
            }
        }
    }
}

fn draw_circle_outline(buffer: &mut [u32], center_x: i32, center_y: i32, radius: i32, line_color: u32) {
    for y in center_y - radius..=center_y + radius {
        for x in center_x - radius..=center_x + radius {
            let distance_squared = (x - center_x) * (x - center_x) + (y - center_y) * (y - center_y);
            if (distance_squared - radius * radius).abs() < radius * 2 {
                set_pixel(buffer, x, y, line_color);
            }
        }
    }
}

// This draws the same lower arc as Java's drawArc(..., 210, 120).
fn draw_smile(buffer: &mut [u32], center_x: i32, center_y: i32) {
    for degrees in 210..=330 {
        let angle = (degrees as f32).to_radians();
        let x = center_x + (30.0 * angle.cos()) as i32;
        let y = center_y - 10 - (20.0 * angle.sin()) as i32;

        for offset_y in -1..=1 {
            for offset_x in -1..=1 {
                set_pixel(buffer, x + offset_x, y + offset_y, RED);
            }
        }
    }
}

// This is the Rust equivalent of Java's paint method.
fn draw_face(buffer: &mut [u32]) {
    buffer.fill(WHITE);

    let center_x = WINDOW_WIDTH as i32 / 2;
    let center_y = WINDOW_HEIGHT as i32 / 2;
    let face_size = 120;
    let face_radius = face_size / 2;

    draw_filled_circle(buffer, center_x, center_y, face_radius, YELLOW);
    draw_circle_outline(buffer, center_x, center_y, face_radius, BLACK);

    draw_filled_circle(buffer, center_x - 15, center_y - 15, 5, BLACK);
    draw_filled_circle(buffer, center_x + 15, center_y - 15, 5, BLACK);

    draw_smile(buffer, center_x, center_y);
}

fn main() {
    // This replaces creating and setting up the Java JFrame.
    let mut window = Window::new(
        "Frogger",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions::default(),
    )
    .expect("Could not create the Frogger window");

    let mut buffer = vec![WHITE; WINDOW_WIDTH * WINDOW_HEIGHT];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        draw_face(&mut buffer);
        window
            .update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .expect("Could not draw the Frogger window");
    }
}
