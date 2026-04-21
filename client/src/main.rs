// WARN: Comment this after building out everything
#![allow(dead_code, unused)]
mod chunk;
mod chunk_batch;
mod chunk_loader;
mod config;
mod feature_stamp;
mod menu;
mod network;
mod planet;
mod player;
mod remote_player;
mod shaders;
mod terrain_manager;
mod terrain_shaper;
mod time_of_day;
mod utils;
mod world;

use crate::utils::draw_stats;
use menu::{AppState, MenuAction, MenuState};
use network::*;
use player::Player;
use raylib::prelude::*;
use remote_player::RemotePlayer;
use shaders::ShaderManager;
use std::collections::HashMap;
use sysinfo::System;
use terrain_manager::TerrainManager;
use time_of_day::TimeOfDay;
use uuid::Uuid;

use config::*;

use crate::shaders::{FogConfig, SunConfig};

fn main() {
    // Network setup — spawned once; stays alive for process lifetime
    let server_config = ServerConfig::default();
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
    // Cursor starts enabled for the menu; disabled when entering 3D world
    rl_handle.set_exit_key(None);

    // Aspect ratio for proper frustum culling
    let aspect_ratio = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;

    // WARN: Raylib wrapper does not provide a wrapper to modify clip plane
    // (all calls to raylib-rs are "unsafe" as Raylib is in C)
    unsafe {
        raylib::ffi::rlSetClipPlanes(0.01, FAR_CLIP_PLANE_DISTANCE as f64);
    }

    // Shader setup
    let mut shader_manager = ShaderManager::new();
    shader_manager.load_shaders(&mut rl_handle, &rl_thread);

    // App state
    let mut app_state = AppState::Menu;
    let mut menu = MenuState::new();
    let menu_seed: u64 = 12345;
    // Server seed received before entering world is applied via reinit_with_seed
    let mut queued_server_seed: Option<u64> = None;

    // Deferred until user enters the world
    let mut terrain_manager: Option<TerrainManager> = None;
    let mut time_of_day: Option<TimeOfDay> = None;
    let mut player: Option<Player> = None;
    let mut last_position_update = 0.0;

    // Main loop
    while !rl_handle.window_should_close() {
        let dt = rl_handle.get_frame_time();

        match app_state {
            AppState::Menu => {
                // Drain network events — stash any server seed for when we enter the world
                while let Ok(event) = network_events.try_recv() {
                    if let NetworkEvent::WorldSync { seed, .. } = event {
                        queued_server_seed = Some(seed);
                    }
                }

                match menu.handle_input(&mut rl_handle, &rl_thread, menu_seed) {
                    MenuAction::EnterWorld => {
                        // Free preview texture GPU memory before creating terrain
                        menu.preview_texture = None;

                        let seed = queued_server_seed.unwrap_or(menu_seed);
                        let tm = TerrainManager::new_with_seed(
                            seed,
                            menu.selected_planet.clone(),
                        );
                        time_of_day = Some(TimeOfDay::new(tm.planet.sky_color));
                        let center = (tm.planet.grid_size as f32 / 2.0)
                            * (CHUNK_SIZE as f32 * TERRAIN_RESOLUTION);
                        player = Some(Player::new(Vector3::new(center, 500.0, center)));
                        terrain_manager = Some(tm);
                        rl_handle.disable_cursor();
                        app_state = AppState::Loading;
                    }
                    MenuAction::Quit => break,
                    MenuAction::None => {}
                }

                let mut d = rl_handle.begin_drawing(&rl_thread);
                d.clear_background(Color::BLACK);
                menu.draw(&mut d, WINDOW_WIDTH, WINDOW_HEIGHT);
            }

            AppState::Loading => {
                // Process network events — apply server seed if it arrives during load
                while let Ok(event) = network_events.try_recv() {
                    if let NetworkEvent::WorldSync { seed, hour } = event {
                        time_of_day.as_mut().unwrap().set_hour(hour);
                        terrain_manager.as_mut().unwrap().reinit_with_seed(seed);
                    }
                }

                let terrain_shader = shader_manager.get_terrain_shader();
                let tm = terrain_manager.as_mut().unwrap();
                tm.update(
                    player.as_ref().unwrap().position,
                    &mut rl_handle,
                    &rl_thread,
                    terrain_shader,
                );

                if tm.is_preload_complete() {
                    app_state = AppState::Playing;
                    continue;
                }

                let progress = tm.preload_progress();
                let mut d = rl_handle.begin_drawing(&rl_thread);
                d.clear_background(Color::BLACK);
                d.draw_rectangle(
                    200,
                    WINDOW_HEIGHT / 2 - 10,
                    WINDOW_WIDTH - 400,
                    20,
                    Color::DARKGRAY,
                );
                let fill = ((WINDOW_WIDTH - 400) as f32 * progress) as i32;
                d.draw_rectangle(200, WINDOW_HEIGHT / 2 - 10, fill, 20, Color::GREEN);
                d.draw_text(
                    &format!("Generating planet... {:.0}%", progress * 100.0),
                    200,
                    WINDOW_HEIGHT / 2 - 40,
                    24,
                    Color::WHITE,
                );
            }

            AppState::Playing => {
                // Process all network events
                while let Ok(event) = network_events.try_recv() {
                    match event {
                        NetworkEvent::Connected => {
                            println!("Connected to server!");
                        }
                        NetworkEvent::Disconnected => {
                            println!("Disconnected from server");
                            remote_players.clear();
                        }
                        NetworkEvent::PlayerPositionUpdate { player_id, position } => {
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
                            time_of_day.as_mut().unwrap().set_hour(hour);
                            terrain_manager.as_mut().unwrap().reinit_with_seed(seed);
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

                let player = player.as_mut().unwrap();
                let terrain_manager = terrain_manager.as_mut().unwrap();
                let time_of_day = time_of_day.as_mut().unwrap();

                // Terrain updating
                let terrain_shader = shader_manager.get_terrain_shader();
                terrain_manager.update(player.position, &mut rl_handle, &rl_thread, terrain_shader);

                // Advance time
                time_of_day.advance(dt, TIME_SPEED_20_MIN);

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
                player.update(&rl_handle, terrain_manager, dt);

                // Calculate fog settings based on actual view distance
                let fog_distances = terrain_manager.get_fog_distances();
                let fog_config =
                    FogConfig::new(fog_distances.0, fog_distances.1, time_of_day.fog_color());

                // Sun configuration
                let sun_config = SunConfig::new(
                    time_of_day.sun_direction(),
                    Color::LIGHTGOLDENRODYELLOW,
                    time_of_day.sun_intensity(),
                    time_of_day.ambient_strength(),
                );

                // 2D drawing setup
                let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
                draw_handle.clear_background(time_of_day.sky_color());

                {
                    // 3D drawing
                    let mut draw3d_handle = draw_handle.begin_mode3D(player.get_camera());

                    terrain_manager.render(
                        &mut draw3d_handle,
                        &player.get_camera(),
                        aspect_ratio,
                        &shader_manager,
                        fog_config,
                        sun_config,
                    );

                    let sun_pos =
                        player.position + time_of_day.sun_direction() * SUN_PLAYER_DISTANCE;
                    if RENDER_WIREFRAME {
                        draw3d_handle.draw_sphere_wires(
                            sun_pos,
                            SUN_RADIUS,
                            10,
                            10,
                            Color::BLACK,
                        );
                    } else {
                        draw3d_handle.draw_sphere(
                            sun_pos,
                            SUN_RADIUS,
                            Color::LIGHTGOLDENRODYELLOW,
                        );
                    }

                    for remote_player in remote_players.values() {
                        remote_player.render(&mut draw3d_handle);
                    }
                }

                draw_stats(
                    &mut draw_handle,
                    player,
                    used_mb,
                    terrain_manager,
                    time_of_day,
                );
            }
        }
    }
}
