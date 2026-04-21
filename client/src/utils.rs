use crate::Player;
use crate::TerrainManager;
use crate::TimeOfDay;
use crate::config::*;
use raylib::prelude::*;

pub fn color_to_f32(color: Color) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

pub fn rl_to_primitive_vec3(vec: Vector3) -> [f32; 3] {
    [vec.x, vec.y, vec.z]
}

pub fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

pub fn draw_stats(
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
        &format!("Seed: {}", terrain_manager.planet.seed),
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
