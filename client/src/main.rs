// WARN: REMOVE THESE ALLOWS
#![allow(dead_code, unused)]
mod player;
mod terrain;
mod world;

use player::Player;
use raylib::prelude::*;
use terrain::Terrain;

fn main() {
    let (mut rl_handle, rl_thread) = raylib::init()
        .size(1280, 720)
        .title("Terrain Explorer")
        .build();

    rl_handle.set_target_fps(60);
    rl_handle.disable_cursor();

    // Player and terrain setup
    let mut player = Player::new(Vector3::new(0.0, 25.0, 50.0));
    let terrain = Terrain::new(&mut rl_handle, &rl_thread, 12345);

    // Main loop
    while !rl_handle.window_should_close() {
        // Update
        let dt = rl_handle.get_frame_time();
        player.update(&rl_handle, &terrain, dt);

        let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
        draw_handle.clear_background(Color::DEEPSKYBLUE);

        // 3D drawing
        {
            let mut draw3d_handle = draw_handle.begin_mode3D(player.get_camera());
            terrain.render(&mut draw3d_handle);
        }

        draw_handle.draw_fps(10, 10);
    }
}
