// WARN: REMOVE THESE ALLOWS
#![allow(dead_code, unused)]
mod chunk;
mod player;
mod shaders;
mod terrain_manager;
mod world;

use player::Player;
use raylib::prelude::*;
use shaders::ShaderManager;
use sysinfo::System;
use terrain_manager::TerrainManager;

fn main() {
    // sysinfo for memory alloc debugging
    let mut sys = System::new_all();
    let current_pid = sysinfo::get_current_pid().expect("Failed to get PID");

    // Raylib setup
    let (mut rl_handle, rl_thread) = raylib::init()
        .size(1280, 720)
        .title("Terrain Explorer")
        .build();

    rl_handle.set_target_fps(60);
    rl_handle.disable_cursor();

    // Player and terrain setup
    let mut player = Player::new(Vector3::new(0.0, 100.0, 0.0));
    let mut terrain_manager = TerrainManager::new(12345);

    // Shader setup
    let mut shader_manager = ShaderManager::new();
    shader_manager.load_shaders(&mut rl_handle, &rl_thread);

    // Main loop
    while !rl_handle.window_should_close() {
        // Process memory usage
        sys.refresh_all();
        let used_mb = if let Some(process) = sys.process(current_pid) {
            process.memory() as f32 / 1024.0 / 1024.0
        } else {
            0.0
        };

        // Update chunks based on player pos
        terrain_manager.update(player.position, &mut rl_handle, &rl_thread);

        // Update player
        let dt = rl_handle.get_frame_time();
        player.update(&rl_handle, &terrain_manager, dt);

        // Calculate fog settings based on actual view distance
        let (fog_near, fog_far) = terrain_manager.get_fog_distances();

        // 2d drawing setup before 3d
        let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
        draw_handle.clear_background(Color::DEEPSKYBLUE);

        {
            // 3D drawing
            let mut draw3d_handle = draw_handle.begin_mode3D(player.get_camera());
            terrain_manager.render(
                &mut draw3d_handle,
                &player.get_camera(),
                shader_manager.get_fog_shader(),
                fog_near,
                fog_far,
                Color::RED, // Temporary test color
            );
        }

        draw_stats(&mut draw_handle, &player, used_mb, &terrain_manager);
    }
}

fn draw_stats(
    d: &mut RaylibDrawHandle,
    player: &Player,
    used_mb: f32,
    terrain_manager: &TerrainManager,
) {
    // FPS
    d.draw_fps(10, 10);

    // Player pos
    d.draw_text(
        &format!(
            "x:{} y:{} z:{}",
            player.position.x.round_ties_even(),
            player.position.y.round_ties_even(),
            player.position.z.round_ties_even()
        ),
        10,
        30,
        20,
        Color::BLACK,
    );

    // Grounded stae
    d.draw_text(
        &format!("Grounded: {}", player.is_grounded),
        10,
        50,
        20,
        Color::BLACK,
    );

    // Memory usage
    d.draw_text(
        &format!("Memory: {:.1} MB", used_mb),
        10,
        70,
        20,
        Color::BLACK,
    );

    // Rendered versus mapped chunks
    d.draw_text(
        &format!(
            "Chunks: {}/{}",
            terrain_manager.rendered_chunk_count(),
            terrain_manager.chunk_count()
        ),
        10,
        90,
        20,
        Color::BLACK,
    );
}
