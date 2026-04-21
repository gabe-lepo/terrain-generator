#![allow(dead_code, unused)]

mod config;
mod menu;
mod noise;
mod utils;

use config::*;
use menu::MenuAction;
use noise::*;
use raylib::prelude::*;
use utils::*;

fn main() {
    // Raylib setup
    let (mut rl_handle, rl_thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("terrain playground")
        .build();

    rl_handle.set_target_fps(120);
    rl_handle.set_trace_log(TraceLogLevel::LOG_WARNING);
    rl_handle.set_exit_key(None);

    // Camera setup
    let mut camera = Camera2D {
        offset: Vector2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0),
        target: Vector2::zero(),
        rotation: 0.0,
        zoom: 1.0,
    };

    // Noise setup
    let noise_params = NoiseParams::new();
    let continent_params = ContinentParams::new();
    let redistribution_params = RedistributionParams::new();
    let moisture_params = MoistureParams::new();
    let mut noise_config = NoiseConfig::new(
        noise_params,
        continent_params,
        redistribution_params,
        moisture_params,
    );

    // Map setup
    let mut seed = utils::get_rand_seed();
    let mut grid_size: usize = DEFAULT_GRID_SIZE;
    let (mut heightmap, mut moisturemap) = noise::generate_map(&noise_config, seed, grid_size);
    let mut map_texture = bake_map_texture(
        &mut rl_handle,
        &rl_thread,
        &heightmap,
        &moisturemap,
        grid_size,
        noise_config.use_moisture,
        noise_config.do_lerp,
    );

    // Menu setup
    let mut menu = menu::MenuState::new();

    // Main loop
    while !rl_handle.window_should_close() {
        let dt = rl_handle.get_frame_time();

        // Input handling and possible regen
        let camera_regen = handle_inputs_and_regen(&rl_handle, &mut camera, dt);
        let menu_action = menu.handle_input(&rl_handle, &mut noise_config, &mut grid_size, dt);
        if camera_regen {
            seed = get_rand_seed();
            (heightmap, moisturemap) = noise::generate_map(&noise_config, seed, grid_size);
            map_texture = bake_map_texture(
                &mut rl_handle,
                &rl_thread,
                &heightmap,
                &moisturemap,
                grid_size,
                noise_config.use_moisture,
                noise_config.do_lerp,
            );
        } else {
            match menu_action {
                MenuAction::Regen => {
                    (heightmap, moisturemap) = noise::generate_map(&noise_config, seed, grid_size);
                    map_texture = bake_map_texture(
                        &mut rl_handle,
                        &rl_thread,
                        &heightmap,
                        &moisturemap,
                        grid_size,
                        noise_config.use_moisture,
                        noise_config.do_lerp,
                    );
                }
                MenuAction::Rebake => {
                    map_texture = bake_map_texture(
                        &mut rl_handle,
                        &rl_thread,
                        &heightmap,
                        &moisturemap,
                        grid_size,
                        noise_config.use_moisture,
                        noise_config.do_lerp,
                    );
                }
                MenuAction::None => {}
            }
        }

        // Drawing
        let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
        draw_handle.clear_background(Color::BLACK);

        {
            let mut draw2d_handle = draw_handle.begin_mode2D(camera);
            let map_px =
                (grid_size as i32 * (WINDOW_WIDTH.min(WINDOW_HEIGHT) / grid_size as i32)) / 2;
            let map_world_size = map_px * 2;
            draw2d_handle.draw_texture_pro(
                &map_texture,
                Rectangle::new(0.0, 0.0, grid_size as f32, grid_size as f32),
                Rectangle::new(
                    -map_px as f32,
                    -map_px as f32,
                    map_world_size as f32,
                    map_world_size as f32,
                ),
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        }

        draw_text(&mut draw_handle, &seed, &camera);
        menu.draw(&mut draw_handle, &noise_config, grid_size);
    }
}

fn bake_map_texture(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    heightmap: &Vec<Vec<f64>>,
    moisturemap: &Vec<Vec<f64>>,
    grid_size: usize,
    use_moisture: bool,
    do_lerp: bool,
) -> Texture2D {
    let mut image = Image::gen_image_color(grid_size as i32, grid_size as i32, Color::BLACK);
    for x in 0..grid_size {
        for y in 0..grid_size {
            let color = biome_color(heightmap[x][y], moisturemap[x][y], use_moisture, do_lerp);
            image.draw_pixel(x as i32, y as i32, color);
        }
    }
    rl.load_texture_from_image(thread, &image)
        .expect("Failed to load map texture from image")
}
