// WARN: Comment this after building out everything
#![allow(dead_code, unused)]
mod chunk;
mod chunk_loader;
mod config;
mod network;
mod planet;
mod player;
mod remote_player;
mod shaders;
mod terrain_manager;
mod time_of_day;
mod utils;
mod world;

use config::{
    CONNECT, FAR_CLIP_PLANE_DISTANCE, RENDER_WIREFRAME, SUNRISE_START, SUNSET_END,
    TIME_SPEED_10_MIN, TIME_SPEED_DEBUG, TIME_STARTING_HOUR, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use network::{
    NetworkEvent, ServerConfig, round_position, should_send_position_update, spawn_network_task,
};
use player::Player;
use raylib::prelude::*;
use remote_player::RemotePlayer;
use shaders::ShaderManager;
use std::collections::HashMap;
use sysinfo::System;
use terrain_manager::TerrainManager;
use time_of_day::TimeOfDay;
use uuid::Uuid;

fn main() {
    // Network setup
    let server_config = ServerConfig::default(); // Will configure later by user
    let (network_handle, mut network_events) = spawn_network_task(server_config);
    let mut remote_players: HashMap<Uuid, RemotePlayer> = HashMap::new();

    // sysinfo for memory alloc debugging
    let mut sys = System::new_all();
    let current_pid = sysinfo::get_current_pid().expect("Failed to get PID");

    // Raylib setup
    let (mut rl_handle, rl_thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Terrain Explorer")
        .build();

    rl_handle.set_target_fps(120);
    rl_handle.set_trace_log(TraceLogLevel::LOG_WARNING);
    rl_handle.disable_cursor();
    rl_handle.set_exit_key(None);

    // WARN: Experimental camera far plane clip modification
    unsafe {
        raylib::ffi::rlSetClipPlanes(0.01, FAR_CLIP_PLANE_DISTANCE as f64);
    }

    // Player setup
    let mut player = Player::new(Vector3::new(-705.0, 50.0, 227.0));

    // Shader setup
    let mut shader_manager = ShaderManager::new();
    shader_manager.load_shaders(&mut rl_handle, &rl_thread);

    // timers
    let mut last_position_update = 0.0;

    // Time of day setup
    let mut time_of_day = TimeOfDay::new(TIME_STARTING_HOUR);

    // Terrain manager setup
    let mut terrain_manager = TerrainManager::new(0);
    if !CONNECT {
        terrain_manager.reinit_with_seed(12345);
    }

    // Main loop
    while !rl_handle.window_should_close() {
        // Process network events
        while let Ok(event) = network_events.try_recv() {
            match event {
                NetworkEvent::Connected => {
                    println!("Conneceted to server!");
                }
                NetworkEvent::Disconnected => {
                    println!("Disconnected from server");
                    remote_players.clear();
                }
                NetworkEvent::PlayerPositionUpdate {
                    player_id,
                    position,
                } => {
                    remote_players
                        .entry(player_id)
                        .and_modify(|p| p.update_position(position))
                        .or_insert_with(|| RemotePlayer::new(player_id, position));
                }
                NetworkEvent::PlayerDisconnected { player_id } => {
                    remote_players.remove(&player_id);
                    println!("Player {} disconnected", player_id);
                }
                NetworkEvent::WorldSync { seed, hour } => {
                    time_of_day.set_hour(hour);
                    terrain_manager.reinit_with_seed(seed);
                    println!("World synced: Seed: {} | Hour: {:.2}", seed, hour);
                }
            }
        }

        // Process memory usage
        sys.refresh_all();
        let used_mb = if let Some(process) = sys.process(current_pid) {
            process.memory() as f32 / 1024.0 / 1024.0
        } else {
            0.0
        };

        // Update chunks based on player pos (pass fog shader so new chunks get it applied)
        let fog_shader = shader_manager.get_terrain_shader();
        terrain_manager.update(player.position, &mut rl_handle, &rl_thread, fog_shader);

        let dt = rl_handle.get_frame_time();

        // Advance time
        time_of_day.advance(dt, TIME_SPEED_10_MIN);

        // Send position to server at configured rate
        if should_send_position_update(&mut last_position_update, dt) {
            let rounded_pos = round_position(shared::NetworkVec3::new(
                player.position.x,
                player.position.y,
                player.position.z,
            ));
            network_handle.send_position_update(rounded_pos);
        }

        // Update player
        player.update(&rl_handle, &terrain_manager, dt);

        // Calculate fog settings based on actual view distance
        let (fog_near, fog_far) = terrain_manager.get_fog_distances();

        // 2d drawing setup before 3d
        let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
        draw_handle.clear_background(time_of_day.sky_color());

        {
            // 3D drawing
            let mut draw3d_handle = draw_handle.begin_mode3D(player.get_camera());

            terrain_manager.render(
                &mut draw3d_handle,
                &player.get_camera(),
                &shader_manager,
                fog_near,
                fog_far,
                time_of_day.fog_color(),
                time_of_day.sun_direction(),
                Color::LIGHTGOLDENRODYELLOW,
                time_of_day.sun_intensity(),
                time_of_day.ambient_strength(),
            );

            // Temporary static sun ball thing
            let sun_pos = player.position + time_of_day.sun_direction() * 5000.0;
            if RENDER_WIREFRAME {
                draw3d_handle.draw_sphere_wires(sun_pos, 1000.0, 10, 10, Color::BLACK);
            } else {
                draw3d_handle.draw_sphere(sun_pos, 1000.0, Color::LIGHTGOLDENRODYELLOW);
            }

            // Render remote players
            for remote_player in remote_players.values() {
                remote_player.render(&mut draw3d_handle);
            }
        }

        draw_stats(
            &mut draw_handle,
            &player,
            used_mb,
            &terrain_manager,
            &time_of_day,
        );
    }
}

fn draw_stats(
    d: &mut RaylibDrawHandle,
    player: &Player,
    used_mb: f32,
    terrain_manager: &TerrainManager,
    time_of_day: &TimeOfDay,
) {
    let text_color = if time_of_day.hour() < SUNRISE_START || time_of_day.hour() > SUNSET_END {
        Color::WHITE
    } else {
        Color::BLACK
    };

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
        text_color,
    );

    // Memory usage
    d.draw_text(
        &format!("Memory: {:.1} MB", used_mb),
        10,
        50,
        20,
        text_color,
    );

    // Rendered versus mapped chunks
    d.draw_text(
        &format!(
            "Chunks: {}/{}",
            terrain_manager.rendered_chunk_count(),
            terrain_manager.chunk_count()
        ),
        10,
        70,
        20,
        text_color,
    );

    // Planet type
    d.draw_text(
        terrain_manager.planet.get_planet_name(),
        10,
        90,
        20,
        text_color,
    );

    // Seed
    d.draw_text(
        &format!("Seed: {}", terrain_manager.planet.seed.to_string()),
        10,
        110,
        20,
        text_color,
    );

    // Time of day
    d.draw_text(
        &format!("Time: {:.2}", time_of_day.hour()),
        10,
        130,
        20,
        text_color,
    );
}
