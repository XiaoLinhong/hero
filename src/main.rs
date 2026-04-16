use std::fs;

use macroquad::prelude::*;
use macroquad_particles::{self as particles, ColorCurve, Emitter, EmitterConfig};

const MOVEMENT_SPEED: f32 = 200.0;
const FILE_NAME: &str = "best.txt";

const FRAGMENT_SHADER: &str = include_str!("starfield-shader.glsl"); // 片段着色器

const VERTEX_SHADER: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying float iTime;

uniform mat4 Model;
uniform mat4 Projection;
uniform vec4 _Time;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    iTime = _Time.x;
}
";

struct Shape {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    speed: f32,
    alive: bool,
}

impl Shape {
    fn collides_with(&self, other: &Shape) -> bool {
        self.rect().overlaps(&other.rect())
    }

    fn rect(&self) -> Rect {
        Rect {
            x: self.x - self.w / 2.0,
            y: self.y - self.h / 2.0,
            w: self.w,
            h: self.h,
        }
    }
}

enum GameMode {
    MainMenu,
    Playing,
    Paused,
    GameOver,
}

struct GameWorld {
    mode: GameMode,
    score: usize,
    best_score: usize,
    player: Shape,
    bullets: Vec<Shape>,
    enemies: Vec<Shape>,
    explosions: Vec<(Emitter, Vec2)>,
    direction_modifier: f32,
}

impl GameWorld {
    fn new() -> Self {
        let best = fs::read_to_string(FILE_NAME)
            .map(|s| s.parse().unwrap())
            .unwrap_or(0);

        Self {
            mode: GameMode::MainMenu,
            score: 0,
            best_score: best,
            player: Shape {
                x: screen_width() / 2.0,
                y: screen_height() / 2.0,
                w: 32.0,
                h: 32.0,
                speed: MOVEMENT_SPEED,
                alive: true,
            },
            bullets: vec![],
            enemies: vec![],
            explosions: vec![],
            direction_modifier: 0.0,
        }
    }

    fn reset(&mut self) {
        self.score = 0;
        self.enemies.clear();
        self.bullets.clear();
        self.explosions.clear();
        self.player.x = screen_width() / 2.0;
        self.player.y = screen_height() / 2.0;
        self.mode = GameMode::Playing;
    }
}

// --- 核心逻辑 ---

#[macroquad::main("Hero")]
async fn main() {
    // 初始化渲染器信息
    // let ctx = unsafe { get_internal_gl().quad_context };
    // println!("Renderer: {:?}", ctx.info());

    rand::srand(miniquad::date::now() as u64);
    let mut world = GameWorld::new();

    // 着色器相关
    let render_target = render_target(320, 150);
    render_target.texture.set_filter(FilterMode::Nearest);
    let material: Material = load_material(
        ShaderSource::Glsl {
            vertex: VERTEX_SHADER,
            fragment: FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("iResolution", UniformType::Float2),
                UniformDesc::new("direction_modifier", UniformType::Float1),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    loop {
        clear_background(BLACK);
        // 着色器相关
        material.set_uniform("iResolution", (screen_width(), screen_height()));
        material.set_uniform("direction_modifier", world.direction_modifier);
        gl_use_material(&material);
        draw_texture_ex(
            &render_target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
        gl_use_default_material();

        match world.mode {
            GameMode::MainMenu => update_main_menu(&mut world),
            GameMode::Playing => update_playing(&mut world),
            GameMode::Paused => update_paused(&mut world),
            GameMode::GameOver => update_game_over(&mut world),
        }

        draw_ui(&world);

        next_frame().await;
    }
}

// --- 子状态处理函数 ---

fn update_main_menu(world: &mut GameWorld) {
    if is_key_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
    if is_key_pressed(KeyCode::Space) {
        world.reset();
    }

    draw_text(&format!("score: {}", get_fps()), 20.0, 20.0, 30.0, WHITE);

    draw_centered_text("Press Space to start game!", 50.0, RED);
}

fn update_playing(world: &mut GameWorld) {
    let dt = get_frame_time();

    // 1. 处理输入
    handle_player_input(world, dt);

    if is_key_pressed(KeyCode::A) && world.bullets.len() < 5 {
        world.bullets.push(Shape {
            w: 4.0,
            h: 10.0,
            speed: -100.0,
            x: world.player.x,
            y: world.player.y - world.player.h / 2.0,
            alive: true,
        });
    }

    if is_key_pressed(KeyCode::Escape) {
        world.mode = GameMode::Paused;
    }

    // 2. 生成敌人
    if rand::gen_range(0, 99) >= 90 {
        let size = rand::gen_range(16.0, 32.0);
        world.enemies.push(Shape {
            w: size,
            h: size,
            speed: rand::gen_range(50.0, 150.0),
            x: rand::gen_range(size / 2.0, screen_width() - size / 2.0),
            y: -size,
            alive: true,
        });
    }

    // 3. 物理更新
    for enemy in &mut world.enemies {
        enemy.y += enemy.speed * dt;
    }
    for bullet in &mut world.bullets {
        bullet.y += bullet.speed * dt;
    }

    // 4. 碰撞检测
    for bullet in &mut world.bullets {
        for enemy in &mut world.enemies {
            if enemy.collides_with(bullet) {
                enemy.alive = false;
                bullet.alive = false;
                world.score += ((enemy.w * enemy.h) / 10.0) as usize;

                world.explosions.push((
                    Emitter::new(EmitterConfig {
                        amount: enemy.w.round() as u32 * 2,
                        ..particle_explosion()
                    }),
                    vec2(enemy.x, enemy.y),
                ));
            }
        }
    }

    if world.score > world.best_score {
        world.best_score = world.score;
        fs::write(FILE_NAME, world.best_score.to_string()).ok();
    }

    if world.enemies.iter().any(|e| e.collides_with(&world.player)) {
        world.mode = GameMode::GameOver;
    }

    // 5. 清理对象
    world.enemies.retain(|e| e.y < screen_height() + e.h && e.alive);
    world.bullets.retain(|b| b.y > 0.0 && b.alive);
    world.explosions.retain(|(explosion, _)| explosion.config.emitting);
    // 6. 渲染游戏物体
    draw_world_entities(world);
}

fn update_paused(world: &mut GameWorld) {
    if is_key_pressed(KeyCode::Space) {
        world.mode = GameMode::Playing;
    }
    if is_key_pressed(KeyCode::Escape) {
        world.mode = GameMode::MainMenu;
    }

    draw_world_entities(world); // 暂停时也绘制背景物体
    draw_centered_text("Press Space to continue!", 50.0, RED);
}

fn update_game_over(world: &mut GameWorld) {
    if is_key_pressed(KeyCode::Escape) {
        world.mode = GameMode::MainMenu;
    }
    draw_world_entities(world); // 暂停时也绘制背景物体
    draw_centered_text("GAME OVER!", 50.0, RED);
}

fn handle_player_input(world: &mut GameWorld, dt: f32) {
    if is_key_down(KeyCode::Right) { 
        world.player.x += MOVEMENT_SPEED * dt;
        world.direction_modifier += 0.05 * dt;
    }
    if is_key_down(KeyCode::Left) { world.player.x -= MOVEMENT_SPEED * dt; }
    if is_key_down(KeyCode::Down) { world.player.y += MOVEMENT_SPEED * dt; }
    if is_key_down(KeyCode::Up) { world.player.y -= MOVEMENT_SPEED * dt; }

    world.player.x = clamp(world.player.x, 0.0, screen_width());
    world.player.y = clamp(world.player.y, 0.0, screen_height());
}

fn draw_world_entities(world: &mut GameWorld) {
    draw_circle(world.player.x, world.player.y, world.player.w / 2.0, YELLOW);
    
    for enemy in &world.enemies {
        draw_rectangle(enemy.x - enemy.w / 2.0, enemy.y - enemy.h / 2.0, enemy.w, enemy.h, GREEN);
    }
    
    for bullet in &world.bullets {
        draw_rectangle(bullet.x - bullet.w / 2.0, bullet.y - bullet.h / 2.0, bullet.w, bullet.h, BEIGE);
    }

    for (explosion, coords) in world.explosions.iter_mut() {
                    explosion.draw(*coords);
    }
}

fn draw_ui(world: &GameWorld) {
    draw_text(&format!("score: {}", world.score), 20.0, 20.0, 30.0, WHITE);
    
    let best_text = format!("best score: {}", world.best_score);
    let dims = measure_text(&best_text, None, 30, 1.0);
    draw_text(&best_text, screen_width() - dims.width - 20.0, 20.0, 30.0, WHITE);
}

fn draw_centered_text(text: &str, size: f32, color: Color) {
    let dims = measure_text(text, None, size as u16, 1.0);
    draw_text(
        text,
        screen_width() / 2.0 - dims.width / 2.0,
        screen_height() / 2.0,
        size,
        color,
    );
}

fn particle_explosion() -> EmitterConfig {
    EmitterConfig {
        local_coords: false,
        one_shot: true,
        emitting: true,
        lifetime: 0.6,
        lifetime_randomness: 0.3,
        explosiveness: 0.65,
        initial_direction_spread: 2.0 * std::f32::consts::PI,
        initial_velocity: 300.0,
        initial_velocity_randomness: 0.8,
        size: 3.0,
        size_randomness: 0.3,
        colors_curve: ColorCurve {
            start: RED,
            mid: ORANGE,
            end: RED,
        },
        amount: 100,
        emission_shape: particles::EmissionShape::Sphere { radius: 30. },
        ..Default::default()
    }
}
