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
    let mut player = Player::new(Vector3::new(0.0, 50.0, 0.0));
    let terrain = Terrain::new(&mut rl_handle, &rl_thread, 12345);

    // Main loop
    while !rl_handle.window_should_close() {
        // Update
        let dt = rl_handle.get_frame_time();
        player.update(&rl_handle, &terrain, dt);

        let mut draw_handle = rl_handle.begin_drawing(&rl_thread);
        draw_handle.clear_background(Color::DEEPSKYBLUE);

        {
            // 3D drawing
            let mut draw3d_handle = draw_handle.begin_mode3D(player.get_camera());
            terrain.render(&mut draw3d_handle);
        }

        draw_stats(&mut draw_handle, &player);
    }
}

fn draw_stats(d: &mut RaylibDrawHandle, player: &Player) {
    // FPS
    d.draw_fps(10, 10);

    // Player pos
    d.draw_text(
        format!(
            "x:{} y:{} z:{}",
            player.position.x.round_ties_even(),
            player.position.y.round_ties_even(),
            player.position.z.round_ties_even()
        )
        .as_str(),
        10,
        30,
        20,
        Color::BLACK,
    );

    // Grounded stae
    d.draw_text(
        format!("Grounded: {}", player.is_grounded).as_str(),
        10,
        50,
        20,
        Color::BLACK,
    );
}
