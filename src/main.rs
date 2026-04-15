use macroquad::prelude::*;

const MOVEMENT_SPEED: f32 = 200.0;

#[macroquad::main("Hero")]
async fn main() {

    rand::srand(miniquad::date::now() as u64);

    let mut squares = vec![];
    let mut circle = Shape {
        x: screen_width() / 2.0,
        y: screen_height() / 2.0,
        size: 32.0,
        speed: MOVEMENT_SPEED,
    };

    loop {
        let delta_time = get_frame_time();
        if is_key_down(KeyCode::Right) {
            circle.x += MOVEMENT_SPEED * delta_time;
        }
        if is_key_down(KeyCode::Left) {
            circle.x -= MOVEMENT_SPEED * delta_time;
        }
        if is_key_down(KeyCode::Down) {
            circle.y += MOVEMENT_SPEED * delta_time;
        }
        if is_key_down(KeyCode::Up) {
            circle.y -= MOVEMENT_SPEED * delta_time;
        }

        // Clamp X and Y to be within the screen
        circle.x = clamp(circle.x, 0.0, screen_width());
        circle.y = clamp(circle.y, 0.0, screen_height());

        if rand::gen_range(0, 99) >= 98 {
            let size = rand::gen_range(16.0, 32.0);
            squares.push(Shape {
                size,
                speed: rand::gen_range(50.0, 150.0),
                x: rand::gen_range(size / 2.0, screen_width() - size / 2.0),
                y: -size,
            });
        }

        for square in &mut squares {
            square.y += square.speed * delta_time;
        }

        // Remove squares below bottom of screen
        squares.retain(|square| square.y < screen_height() + square.size);

        clear_background(DARKPURPLE);

        draw_circle(circle.x, circle.y, circle.size / 2.0, YELLOW);
        for square in &squares {
            draw_rectangle(
                square.x - square.size / 2.0,
                square.y - square.size / 2.0,
                square.size,
                square.size,
                GREEN,
            );
        }

        draw_text(&format!("FPS: {}", get_fps()), 20.0, 20.0, 30.0, WHITE);
        next_frame().await;
    }
}


struct Shape {
    x: f32,
    y: f32,
    size: f32,
    speed: f32,
}

