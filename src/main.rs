use std::fs;

use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, Skin};
use macroquad::experimental::animation::{AnimatedSprite, Animation};
use macroquad::experimental::collections::storage;
use macroquad::experimental::coroutines::start_coroutine;

use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound, play_sound_once, set_sound_volume};

use macroquad_particles::{AtlasConfig, Emitter, EmitterConfig};
// use macroquad_particles::{self as particles, ColorCurve, Emitter, EmitterConfig};

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

struct Resources {
    ship_texture: Texture2D,
    bullet_texture: Texture2D,
    explosion_texture: Texture2D,
    enemy_small_texture: Texture2D,
    enemy_medium_texture: Texture2D,
    enemy_big_texture: Texture2D,
    theme_music: Sound,
    sound_laser: Sound,
    sound_explosion: Sound,
    window_background: Image,
    button_background: Image,
    button_clicked_background: Image,
    font: Vec<u8>,
}

impl Resources {
    async fn new() -> Result<Self, macroquad::Error> {
        let ship_texture: Texture2D = load_texture("ship.png").await?;
        ship_texture.set_filter(FilterMode::Nearest);
        let bullet_texture = load_texture("laser-bolts.png")
        .await?;
        bullet_texture.set_filter(FilterMode::Nearest);
    
        let explosion_texture = load_texture("explosion.png").await?;
        let enemy_small_texture = load_texture("enemy-small.png").await?;
        let enemy_medium_texture = load_texture("enemy-medium.png").await?;
        let enemy_big_texture = load_texture("enemy-big.png").await?;

        let theme_music = load_sound("8bit-spaceshooter.ogg").await?;
        let sound_laser = load_sound("laser.wav").await?;
        let sound_explosion = load_sound("explosion.wav").await?;

        let window_background = load_image("window_background.png").await?;
        let button_background = load_image("button_background.png").await?;
        let button_clicked_background = load_image("button_clicked_background.png").await?;
        let font = load_file("atari_games.ttf").await?;

        Ok(Self {ship_texture, bullet_texture, explosion_texture, 
                enemy_small_texture, enemy_medium_texture, enemy_big_texture, 
                theme_music, sound_laser, sound_explosion, window_background, 
                button_background, button_clicked_background, font })
    }

    #[allow(unused)]
    pub async fn load() -> Result<(), macroquad::Error> {
        let resources_loading = start_coroutine(async move {
            let resources = Resources::new().await.unwrap();
            storage::store(resources);
        });

        while !resources_loading.is_done() {
            clear_background(BLACK);
            let text = format!(
                "Loading resources {}",
                ".".repeat(((get_time() * 2.) as usize) % 4)
            );
            draw_text(
                &text,
                screen_width() / 2. - 160.,
                screen_height() / 2.,
                40.,
                WHITE,
            );
            next_frame().await;
        }

        Ok(())
    }
}



struct GameWorld<>{
    mode: GameMode,
    score: usize,
    best_score: usize,
    ship: Shape,
    bullets: Vec<Shape>,
    enemies: Vec<Shape>,
    explosions: Vec<(Emitter, Vec2)>,
    direction_modifier: f32,
    ship_sprite: AnimatedSprite,
    bullet_sprite: AnimatedSprite,
    enemy_small_sprite: AnimatedSprite,
    enemy_medium_sprite: AnimatedSprite,
    enemy_big_sprite: AnimatedSprite,
    // resources: Resources,
}

impl GameWorld {
    async fn new() -> Result<Self, macroquad::Error> {
        let best = fs::read_to_string(FILE_NAME)
            .map(|s| s.parse().unwrap())
            .unwrap_or(0);

        Ok(Self {
            mode: GameMode::MainMenu,
            score: 0,
            best_score: best,
            ship: Shape {
                x: screen_width() / 2.0,
                y: screen_height() / 2.0,
                w: 16.0 * 2.0,
                h: 24.0 * 2.0,
                speed: MOVEMENT_SPEED,
                alive: true,
            },
            bullets: vec![],
            enemies: vec![],
            explosions: vec![],
            direction_modifier: 0.0,
            ship_sprite: create_ship_sprite(),
            bullet_sprite: create_bullet_sprite(),
            enemy_small_sprite : create_enemy_small_sprite (),
            enemy_medium_sprite : create_enemy_medium_sprite (),
            enemy_big_sprite : create_enemy_big_sprite (),
        })
    }

    fn reset(&mut self) {
        self.score = 0;
        self.enemies.clear();
        self.bullets.clear();
        self.explosions.clear();
        self.ship.x = screen_width() / 2.0;
        self.ship.y = screen_height() / 2.0;
        self.mode = GameMode::Playing;
    }
}

// --- 核心逻辑 ---

#[macroquad::main("Hero")]
async fn main() -> Result<(), macroquad::Error> {
    // 初始化渲染器信息
    // let ctx = unsafe { get_internal_gl().quad_context };
    // println!("Renderer: {:?}", ctx.info());

    set_pc_assets_folder("assets");

    let mut world = GameWorld::new().await?;
    Resources::load().await?;
    let resources = storage::get::<Resources>();
    build_textures_atlas();
    // 把这些零散的小图，在程序启动阶段，自动拼接成一张巨大的“总图”

    // // 着色器相关
    // let render_target = render_target(320, 150); 
    // // 开辟了一块缓冲区（Buffer），在这个 320x150 的小画布上画好所有东西。

    // render_target.texture.set_filter(FilterMode::Nearest);
    // // 把一个 320x150 的纹理拉伸到 1920x1080 的屏幕上时。

    let window_style = root_ui()
        .style_builder()
        .background(resources.window_background.clone()) // 376 * 312
        .background_margin(RectOffset::new(32.0, 76.0, 44.0, 20.0))
        .margin(RectOffset::new(0.0, -40.0, 0.0, 0.0))
        .build();

    let button_style = root_ui()
        .style_builder()
        .background(resources.button_background.clone()) // 36* 36
        .background_clicked(resources.button_clicked_background.clone()) // 36*36
        .background_margin(RectOffset::new(16.0, 16.0, 16.0, 16.0))
        .margin(RectOffset::new(16.0, 0.0, -8.0, -8.0))
        .font(&resources.font)
        .unwrap()
        .text_color(WHITE)
        .font_size(64)
        .build();

    let label_style = root_ui()
        .style_builder()
        .font(&resources.font)
        .unwrap()
        .text_color(WHITE)
        .font_size(28)
        .build();

    let ui_skin = Skin {
        window_style,
        button_style,
        label_style,
        ..root_ui().default_skin()
    };
    root_ui().push_skin(&ui_skin);
    let window_size = vec2(370.0, 320.0);

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

    rand::srand(miniquad::date::now() as u64);

    let mut volume = 0.2;
    play_sound(&resources.theme_music,PlaySoundParams { looped: true, volume: volume,});

    loop {
        volume += 0.001;
        volume = volume.min(0.5);
        set_sound_volume(&resources.theme_music, volume);
        clear_background(BLACK);
        // 更新数据：每一帧物体的颜色、时间、光照位置都在变
        material.set_uniform("iResolution", (screen_width(), screen_height()));
        material.set_uniform("direction_modifier", world.direction_modifier);
        // 激活材质：告诉 GPU “从现在开始，用这段 Shader 代码和这些参数来画画”
        gl_use_material(&material);
        // 提交几何体：画出纹理。注意：此时画出的物体会受到上面激活的 Shader 影响
        // draw_texture_ex(
        //     &render_target.texture,
        //     0.,
        //     0.,
        //     WHITE,
        //     DrawTextureParams {
        //         dest_size: Some(vec2(screen_width(), screen_height())),
        //         ..Default::default()
        //     },
        // );
        draw_rectangle(0., 0., screen_width(), screen_height(), WHITE);
        gl_use_default_material(); // 

        match world.mode {
            GameMode::MainMenu => update_main_menu(&mut world, &window_size),
            GameMode::Playing => update_playing(&mut world),
            GameMode::Paused => update_paused(&mut world),
            GameMode::GameOver => update_game_over(&mut world),
        }

        draw_ui(&world);

        next_frame().await;
    }
}

// --- 子状态处理函数 ---

fn update_main_menu(world: &mut GameWorld, window_size: &Vec2) {
    // if is_key_pressed(KeyCode::Escape) {
    //     std::process::exit(0);
    // }
    // if is_key_pressed(KeyCode::Space) {
    //     world.reset();
    // }

    root_ui().window(
        hash!(),
        vec2(
            screen_width() / 2.0 - window_size.x / 2.0,
            screen_height() / 2.0 - window_size.y / 2.0,
        ),
        window_size.clone(),
        |ui| {
            ui.label(vec2(80.0, -34.0), "Main Menu");
            if ui.button(vec2(65.0, 25.0), "Play") {
                world.reset()
            }
            if ui.button(vec2(65.0, 125.0), "Quit") {
                std::process::exit(0);
            }
        },
    );

    draw_text(&format!("score: {}", get_fps()), 20.0, 20.0, 30.0, WHITE);

    // draw_centered_text("Press Space to start game!", 50.0, RED);
}

fn update_playing(world: &mut GameWorld) {
    let dt = get_frame_time();

    let resources = storage::get::<Resources>();

    // 1. 处理输入
    handle_ship_input(world, dt);

    // if is_key_pressed(KeyCode::A) && world.bullets.len() < 5 {
    if is_key_pressed(KeyCode::A) {
        world.bullets.push(Shape {
            w: 32.0,
            h: 32.0,
            speed: -100.0,
            x: world.ship.x,
            y: world.ship.y - 24.0, // 从船头发射
            // y: world.ship.y - world.ship.h / 2.0,
            alive: true,
        });
        play_sound_once(&resources.sound_laser);
    }

    if is_key_pressed(KeyCode::Escape) {
        world.mode = GameMode::Paused;
    }

    // 2. 生成敌人
    if rand::gen_range(0, 99) >= 90 {
        // small: 17, 16;
        // medium: 32, 16;
        // big: 32, 32;
        let w = rand::gen_range(16.0, 32.0);
        let mut h = w*16./17.;
        if w > 21.0 {
            h = w/2.0
        }
        if w > 26.0 {
            h = w/2.0
        }
        world.enemies.push(Shape {
            w,
            h,
            speed: rand::gen_range(50.0, 150.0),
            x: rand::gen_range(w / 2.0, screen_width() - w / 2.0),
            y: -w,
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
    world.ship_sprite.update();
    world.bullet_sprite.update();
    world.enemy_small_sprite.update();

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
                        texture: Some(resources.explosion_texture.clone()),
                        ..particle_explosion()
                    }),
                    vec2(enemy.x, enemy.y),
                ));
                play_sound_once(&resources.sound_explosion);
            }
        }
    }

    if world.score > world.best_score {
        world.best_score = world.score;
        fs::write(FILE_NAME, world.best_score.to_string()).ok();
    }

    if world.enemies.iter().any(|e| e.collides_with(&world.ship)) {
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

fn handle_ship_input(world: &mut GameWorld, dt: f32) {
    world.ship_sprite.set_animation(0);
    if is_key_down(KeyCode::Right) { 
        world.ship.x += MOVEMENT_SPEED * dt;
        world.direction_modifier += 0.05 * dt;
        world.ship_sprite.set_animation(2);
    }
    if is_key_down(KeyCode::Left) { 
        world.ship.x -= MOVEMENT_SPEED * dt; 
        world.direction_modifier -= 0.05 * dt;
        world.ship_sprite.set_animation(1);
    }
    if is_key_down(KeyCode::Down) { world.ship.y += MOVEMENT_SPEED * dt; }
    if is_key_down(KeyCode::Up) { world.ship.y -= MOVEMENT_SPEED * dt; }

    world.ship.x = clamp(world.ship.x, 0.0, screen_width());
    world.ship.y = clamp(world.ship.y, 0.0, screen_height());
}

fn draw_world_entities(world: &mut GameWorld) {
    let resources = storage::get::<Resources>();
    // draw_circle(world.ship.x, world.ship.y, world.ship.w / 2.0, YELLOW);
    let ship_frame  = world.ship_sprite.frame();
    draw_texture_ex(
        &resources.ship_texture,
        world.ship.x - world.ship.w/2.0,
        world.ship.y - world.ship.h/2.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(world.ship.w, world.ship.h)),
            source: Some(ship_frame.source_rect),
            ..Default::default()
        },
    );

    let bullet_frame  = world.bullet_sprite.frame();
    for bullet in &world.bullets {
        draw_texture_ex(
            &resources.bullet_texture,
            bullet.x - bullet.w / 2.0,
            bullet.y - bullet.h / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(bullet.w, bullet.h)),
                source: Some(bullet_frame.source_rect),
                ..Default::default()
            },
        );
        // draw_rectangle(bullet.x - bullet.w / 2.0, bullet.y - bullet.h / 2.0, bullet.w, bullet.h, BEIGE);
    }

    for enemy in &world.enemies {
        let mut enemy_frame = world.enemy_small_sprite.frame();
        let mut texture = &resources.enemy_small_texture;
        if enemy.w > 21.0 {
            enemy_frame = world.enemy_medium_sprite.frame();
            texture = &resources.enemy_medium_texture;
        }
        if enemy.w > 26.0 {
            enemy_frame = world.enemy_big_sprite.frame();
            texture = &resources.enemy_big_texture;
        }
        draw_texture_ex(
            texture,
            enemy.x - enemy.w / 2.0,
            enemy.y - enemy.h / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(enemy.w, enemy.h)),
                source: Some(enemy_frame.source_rect),
                ..Default::default()
            },
        );
        // draw_rectangle(enemy.x - enemy.w / 2.0, enemy.y - enemy.h / 2.0, enemy.w, enemy.h, GREEN);
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
        lifetime: 0.3,
        lifetime_randomness: 0.2,
        explosiveness: 0.65,
        initial_direction_spread: 2.0 * std::f32::consts::PI,
        initial_velocity: 200.0,
        initial_velocity_randomness: 0.8,
        size: 16.0,
        size_randomness: 0.3,
        atlas: Some(AtlasConfig::new(5, 1, 0..)),
        // amount: 1000,
        // emission_shape: particles::EmissionShape::Sphere { radius: 30. },
        ..Default::default()
    }
}

fn create_enemy_small_sprite() -> AnimatedSprite {
    // 34 * 16
    let enemy_small_sprite = AnimatedSprite::new(
        17,
        16,
        &[Animation {
            name: "enemy_small".to_string(),
            row: 0,
            frames: 2,
            fps: 12,
        }],
        true,
    );
    enemy_small_sprite
}

fn create_enemy_medium_sprite() -> AnimatedSprite {
    // 64 * 16
    let enemy_small_sprite = AnimatedSprite::new(
        32,
        16,
        &[Animation {
            name: "enemy_small".to_string(),
            row: 0,
            frames: 2,
            fps: 12,
        }],
        true,
    );
    enemy_small_sprite
}

fn create_enemy_big_sprite() -> AnimatedSprite {
    // 64 * 32
    let enemy_small_sprite = AnimatedSprite::new(
        32,
        32,
        &[Animation {
            name: "enemy_small".to_string(),
            row: 0,
            frames: 2,
            fps: 12,
        }],
        true,
    );
    enemy_small_sprite
}

fn create_bullet_sprite() -> AnimatedSprite {
    // 32 * 32 
    let mut bullet_sprite =AnimatedSprite::new(
        16,
        16,
        &[
            Animation {
                name: "bullet".to_string(),
                row: 0,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "bolt".to_string(),
                row: 1,
                frames: 2,
                fps: 12,
            },
        ],
        true,
    );
    bullet_sprite.set_animation(1);
    bullet_sprite
}

fn create_ship_sprite() -> AnimatedSprite {
    let ship_sprite = AnimatedSprite::new(
        16,  // 
        24, // 会在texture中导航用到
        &[
            Animation {
                name: "idle".to_string(),
                row: 0,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "left".to_string(),
                row: 2,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "right".to_string(),
                row: 4,
                frames: 2,
                fps: 12,
            },
        ],
        true,
    );
    ship_sprite
}
