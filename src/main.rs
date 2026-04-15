use macroquad::prelude::*;

#[macroquad::main("Hero")]
async fn main() {

    loop {
        clear_background(DARKPURPLE);
        draw_text(&format!("FPS: {}", get_fps()), 20.0, 20.0, 30.0, WHITE);
        next_frame().await;
    }
}
