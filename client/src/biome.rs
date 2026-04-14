use noise::{NoiseFn, Perlin};
use raylib::prelude::*;

const BIOME_FREQ: f64 = 0.0005; // Lower the larger the region

/// Color power:
/// < 1.0: favors peak color
/// = 1.0: even linear transition
/// > 1.0: Favors base color until high up then sharp peak
pub struct BiomeParams {
    pub height_scale: f32,
    pub base_height: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub base_color: Color,
    pub peak_color: Color,
    pub color_transition_power: f32,
}

impl BiomeParams {
    pub fn new(
        height_scale: f32,
        base_height: f32,
        octaves: u32,
        persistence: f32,
        base_color: Color,
        peak_color: Color,
        color_transition_power: f32,
    ) -> Self {
        Self {
            height_scale,
            base_height,
            octaves,
            persistence,
            base_color,
            peak_color,
            color_transition_power,
        }
    }
}

#[derive(Clone)]
pub struct BiomeSystem {
    noise: Perlin,
    seed_offset: f64,
}

impl BiomeSystem {
    pub fn new(noise: Perlin, seed_offset: f64) -> Self {
        Self {
            noise,
            seed_offset: seed_offset + 10000.0,
        }
    }

    /// Sample biome at given world pos
    pub fn get_biome_at(&self, x: f32, z: f32) -> BiomeParams {
        let biome_x = (x as f64) * BIOME_FREQ + self.seed_offset;
        let biome_z = (z as f64) * BIOME_FREQ + self.seed_offset;
        let biome_value = self.noise.get([biome_x, biome_z]);

        // blend biomes based on noise val
        Self::blend_biomes(biome_value)
    }

    // Private

    /// Blend between three biomes based on noise val
    fn blend_biomes(noise_value: f64) -> BiomeParams {
        // TODO: Add more....
        // 3 presets for now
        let mountains = Self::mountains();
        let plains = Self::plains();
        let hills = Self::hills();

        // Map noise to blend weighting
        if noise_value < -0.5 {
            let t = ((noise_value + 1.0) / 0.5) as f32;
            Self::lerp_biomes(&plains, &mountains, t)
        } else if noise_value < 0.0 {
            let t = ((noise_value + 0.5) / 0.5) as f32;
            Self::lerp_biomes(&mountains, &hills, t)
        } else {
            let t = ((noise_value - 0.0) / 1.0) as f32;
            Self::lerp_biomes(&hills, &plains, t)
        }
    }

    /// Lerp between 2 biome param sets
    fn lerp_biomes(a: &BiomeParams, b: &BiomeParams, t: f32) -> BiomeParams {
        BiomeParams {
            height_scale: Self::lerp_f32(a.height_scale, b.height_scale, t),
            base_height: Self::lerp_f32(a.base_height, b.base_height, t),
            octaves: Self::lerp_f32(a.octaves as f32, b.octaves as f32, t).round() as u32,
            persistence: Self::lerp_f32(a.persistence, b.persistence, t),
            base_color: Self::lerp_color(&a.base_color, &b.base_color, t),
            peak_color: Self::lerp_color(&a.peak_color, &b.peak_color, t),
            color_transition_power: Self::lerp_f32(
                a.color_transition_power,
                b.color_transition_power,
                t,
            ),
        }
    }

    /// LERP helpers
    fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    fn lerp_color(a: &Color, b: &Color, t: f32) -> Color {
        Color::new(
            Self::lerp_f32(a.r as f32, b.r as f32, t) as u8,
            Self::lerp_f32(a.g as f32, b.g as f32, t) as u8,
            Self::lerp_f32(a.b as f32, b.b as f32, t) as u8,
            255,
        )
    }

    // Define biome presets as assc funcs
    fn mountains() -> BiomeParams {
        BiomeParams::new(
            300.0,
            75.0,
            6,
            0.5,
            Color::new(100, 100, 100, 255),
            Color::new(240, 240, 255, 255),
            3.5,
        )
    }

    fn hills() -> BiomeParams {
        BiomeParams::new(
            75.0,
            5.0,
            2,
            0.1,
            Color::new(50, 150, 50, 255),
            Color::new(100, 180, 100, 255),
            1.0,
        )
    }

    fn plains() -> BiomeParams {
        BiomeParams::new(
            5.0,
            0.0,
            2,
            0.001,
            Color::new(200, 180, 100, 255),
            Color::new(220, 200, 130, 255),
            0.3,
        )
    }
}
