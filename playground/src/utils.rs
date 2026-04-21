use crate::config::*;
use raylib::prelude::*;

pub fn handle_inputs_and_regen(handle: &RaylibHandle, camera: &mut Camera2D, dt: f32) -> bool {
    // Modifiers
    let modifier = if handle.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
        || handle.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT)
    {
        2.5
    } else {
        1.0
    };

    // Panning
    if handle.is_key_down(KeyboardKey::KEY_W) {
        camera.offset.y += (300.0 * modifier) * dt;
    }
    if handle.is_key_down(KeyboardKey::KEY_S) {
        camera.offset.y -= (300.0 * modifier) * dt;
    }
    if handle.is_key_down(KeyboardKey::KEY_A) {
        camera.offset.x += (300.0 * modifier) * dt;
    }
    if handle.is_key_down(KeyboardKey::KEY_D) {
        camera.offset.x -= (300.0 * modifier) * dt;
    }

    // Reset camera
    if handle.is_key_pressed(KeyboardKey::KEY_R) {
        camera.offset = Vector2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
        camera.target = Vector2::zero();
        camera.zoom = 1.0;
    }

    // Zooming
    let zoom_delta = (1.0 * modifier) * dt;
    let before = handle.get_screen_to_world2D(
        Vector2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0),
        *camera,
    );
    if handle.is_key_down(KeyboardKey::KEY_E) {
        camera.zoom += zoom_delta;
    }
    if handle.is_key_down(KeyboardKey::KEY_Q) {
        camera.zoom = (camera.zoom - zoom_delta).max(0.1);
    }
    let after = handle.get_screen_to_world2D(
        Vector2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0),
        *camera,
    );
    camera.target.x += before.x - after.x;
    camera.target.y += before.y - after.y;

    // Regen map with new seed
    if handle.is_key_pressed(KeyboardKey::KEY_N) {
        return true;
    }
    false
}

pub fn get_rand_seed() -> u64 {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Failed getting epoch time")
        .as_millis() as u64;

    seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

pub fn biome_color(height: f64, moisture: f64, use_moisture: bool, do_lerp: bool) -> Color {
    if height < 0.0 || height > 1.0 {
        eprintln!("biome_color>bad range check");
        return Color::BLACK;
    }

    // Water cells ignore moisture — use elevation bands only.
    let water_bands: &[(f64, Color)] = &[
        (0.00, Color::new(10, 20, 80, 255)),    // deep ocean
        (0.15, Color::new(20, 60, 160, 255)),   // ocean
        (0.30, Color::new(40, 100, 200, 255)),  // shallow water
        (0.38, Color::new(210, 195, 140, 255)), // wet sand (transition)
    ];

    if height < 0.38 {
        return sample_bands(height, water_bands, do_lerp);
    }

    if !use_moisture {
        // Fall back to the original single-axis land bands.
        let land_bands: &[(f64, Color)] = &[
            (0.38, Color::new(210, 195, 140, 255)), // wet sand
            (0.42, Color::new(230, 215, 160, 255)), // dry sand
            (0.50, Color::new(80, 130, 50, 255)),   // lowland grass
            (0.62, Color::new(55, 100, 35, 255)),   // forest
            (0.72, Color::new(90, 80, 60, 255)),    // highland / dirt
            (0.82, Color::new(100, 95, 90, 255)),   // rock
            (0.92, Color::new(150, 148, 145, 255)), // high rock
            (1.00, Color::new(240, 245, 255, 255)), // snow
        ];
        return sample_bands(height, land_bands, do_lerp);
    }

    // Whittaker-style biome lookup: elevation in [0.38, 1.0], moisture in [0, 1].
    // Elevation is split into three zones; moisture into three columns.
    //   low elevation  (0.38–0.62): coast/lowland
    //   mid elevation  (0.62–0.82): temperate/highland
    //   high elevation (0.82–1.00): mountain
    fn zone_color(height: f64, moisture: f64) -> Color {
        if height < 0.62 {
            if moisture < 0.33 {
                Color::new(210, 185, 130, 255) // dry coast — sand / scrub
            } else if moisture < 0.66 {
                Color::new(80, 130, 50, 255) // temperate lowland grass
            } else {
                Color::new(34, 100, 60, 255) // wet lowland — tropical / swamp
            }
        } else if height < 0.82 {
            if moisture < 0.33 {
                Color::new(160, 130, 80, 255) // dry highland — savanna / steppe
            } else if moisture < 0.66 {
                Color::new(55, 100, 35, 255) // temperate forest
            } else {
                Color::new(30, 80, 50, 255) // wet highland — dense rainforest
            }
        } else {
            if moisture < 0.4 {
                Color::new(120, 110, 100, 255) // dry mountain — bare rock
            } else {
                Color::new(240, 245, 255, 255) // snowy peak
            }
        }
    }

    if !do_lerp {
        return zone_color(height, moisture);
    }

    // Lerp: blend across the two elevation boundaries (0.62 and 0.82).
    // Within a zone's interior, no blending — only near the boundary.
    const BLEND: f64 = 0.04;
    if (height - 0.62).abs() < BLEND {
        let t = ((height - (0.62 - BLEND)) / (2.0 * BLEND)).clamp(0.0, 1.0) as f32;
        let a = zone_color(0.62 - BLEND, moisture);
        let b = zone_color(0.62 + BLEND, moisture);
        return a.lerp(b, t);
    }
    if (height - 0.82).abs() < BLEND {
        let t = ((height - (0.82 - BLEND)) / (2.0 * BLEND)).clamp(0.0, 1.0) as f32;
        let a = zone_color(0.82 - BLEND, moisture);
        let b = zone_color(0.82 + BLEND, moisture);
        return a.lerp(b, t);
    }
    zone_color(height, moisture)
}

fn sample_bands(height: f64, bands: &[(f64, Color)], do_lerp: bool) -> Color {
    if !do_lerp {
        for i in 0..bands.len() - 1 {
            let (band_height, band_color) = bands[i];
            let (next_band_height, _) = bands[i + 1];
            if height >= band_height && height <= next_band_height {
                return band_color;
            }
        }
    } else {
        for i in 0..bands.len() - 1 {
            let (t0, c0) = bands[i];
            let (t1, c1) = bands[i + 1];
            if height <= t1 {
                let t = ((height - t0) / (t1 - t0)) as f32;
                return c0.lerp(c1, t);
            }
        }
    }
    bands.last().expect("bands last failed").1
}

pub fn draw_text(d: &mut RaylibDrawHandle, seed: &u64, camera: &Camera2D) {
    const FONT_SIZE: i32 = 20;
    const PAD: i32 = 10;
    const ROW_H: i32 = 20;

    let lines = [
        format!("Seed: {}", seed),
        format!("Offset: x={:.2} y={:.2}", camera.offset.x, camera.offset.y),
        format!("Zoom: {:.2}", camera.zoom),
    ];

    let max_w = lines
        .iter()
        .map(|l| d.measure_text(l, FONT_SIZE))
        .max()
        .unwrap_or(0);

    let panel_w = max_w + PAD * 2;
    let panel_h = 20 + lines.len() as i32 * ROW_H + PAD;
    d.draw_rectangle(0, 0, panel_w, panel_h, Color::new(255, 255, 255, 200));
    d.draw_fps(PAD, PAD);
    for (i, line) in lines.iter().enumerate() {
        d.draw_text(line, PAD, PAD + 20 + i as i32 * ROW_H, FONT_SIZE, Color::BLACK);
    }
}

pub fn normalize_oneone(val: f64) -> f64 {
    (val + 1.0) / 2.0
}
