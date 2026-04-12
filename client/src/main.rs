// WARN: REMOVE THIS ALLOW BEFORE TESTING
// #![allow(dead_code, unused)]
mod player;
mod terrain;

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

    // Camera and player setup
    let mut camera = Camera3D::perspective(
        Vector3::new(0.0, 10.0, 0.0),
        Vector3::new(0.0, 10.0, -1.0),
        Vector3::up(),
        70.0,
    );
    let mut player = Player::new(Vector3::new(0.0, 10.0, 0.0));

    // Terrain
    let terrain = Terrain::new(&mut rl_handle, &rl_thread, 12345);

    // Main loop
    while !rl_handle.window_should_close() {
        // Update
        let dt = rl_handle.get_frame_time();

        let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
        draw_handle.clear_background(Color::DEEPSKYBLUE);

        // 3D drawing
        {
            let mut draw3d_handle = draw_handle.begin_mode3D(camera);
            terrain.render(&mut draw3d_handle);
        }

        draw_handle.draw_fps(10, 10);
    }
}
