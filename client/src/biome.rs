use crate::config::{BIOME_FREQ, DESERT, EXTREME_MOUNTAINS, HILLS, MOUNTAINS, SEED};

use noise::{NoiseFn, Perlin};
use raylib::prelude::*;

/// Color power:
/// < 1.0: favors peak color
/// = 1.0: even linear transition
/// > 1.0: Favors base color until high up then sharp peak
#[derive(Clone)]
pub struct BiomeParams {
    pub name: String,
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
        name: String,
        height_scale: f32,
        base_height: f32,
        octaves: u32,
        persistence: f32,
        base_color: Color,
        peak_color: Color,
        color_transition_power: f32,
    ) -> Self {
        Self {
            name,
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
}

impl BiomeSystem {
    pub fn new(noise: Perlin) -> Self {
        Self { noise }
    }

    /// Sample biome at given world pos
    pub fn get_biome_at(&self, x: f32, z: f32) -> BiomeParams {
        let seed_offset = (SEED as f64 * 1_000.0) + 10_000.0;
        let biome_x = (x as f64) * BIOME_FREQ + seed_offset;
        let biome_z = (z as f64) * BIOME_FREQ + seed_offset;

        // blend biomes based on noise val
        let biome_value = self.noise.get([biome_x, biome_z]);
        Self::blend_biomes(Self::remap_biome_noise(biome_value))
    }

    // Private

    /// Blend between biomes based on noise val
    fn blend_biomes(noise_value: f64) -> BiomeParams {
        // NOTE: Keep sorted by threshold val low -> high
        let blend_points: &[(f64, BiomeParams)] = &[
            (-1.0, Self::desert()),
            (-0.67, Self::hills()),
            (-0.33, Self::mountains()),
            (0.0, Self::extreme_mountains()),
            (0.33, Self::mountains()),
            (0.67, Self::hills()),
            (1.0, Self::desert()),
        ];

        // Find two waypoints that bracket noise val
        for i in 1..blend_points.len() {
            let (lo_thresh, ref lo_biome) = blend_points[i - 1];
            let (hi_thresh, ref hi_biome) = blend_points[i];

            if noise_value <= hi_thresh {
                let t = ((noise_value - lo_thresh) / (hi_thresh - lo_thresh)) as f32;
                return Self::lerp_biomes(lo_biome, hi_biome, t.clamp(0.0, 1.0));
            }
        }

        // Fallback return last biome (shouldnt be reached if waypoint range covers -1,1)
        panic!("ERROR: blend points likely not covering [-1,1] range!");
        blend_points
            .last()
            .expect("Failed last waypoint check")
            .1
            .clone()
    }

    fn remap_biome_noise(v: f64) -> f64 {
        let sign = v.signum();
        sign * v.abs().powf(0.9)
    }

    /// Lerp between 2 biome param sets
    fn lerp_biomes(a: &BiomeParams, b: &BiomeParams, t: f32) -> BiomeParams {
        BiomeParams {
            name: if a.name == b.name {
                a.name.clone()
            } else if t < 0.5 {
                format!("{} -> {}", a.name, b.name)
            } else {
                format!("{} -> {}", b.name, a.name)
            },
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

    // Define biome presets from config
    fn extreme_mountains() -> BiomeParams {
        BiomeParams::new(
            EXTREME_MOUNTAINS.name.to_string(),
            EXTREME_MOUNTAINS.height_scale,
            EXTREME_MOUNTAINS.base_height,
            EXTREME_MOUNTAINS.octaves,
            EXTREME_MOUNTAINS.persistence,
            EXTREME_MOUNTAINS.base_color,
            EXTREME_MOUNTAINS.peak_color,
            EXTREME_MOUNTAINS.color_power,
        )
    }

    fn mountains() -> BiomeParams {
        BiomeParams::new(
            MOUNTAINS.name.to_string(),
            MOUNTAINS.height_scale,
            MOUNTAINS.base_height,
            MOUNTAINS.octaves,
            MOUNTAINS.persistence,
            MOUNTAINS.base_color,
            MOUNTAINS.peak_color,
            MOUNTAINS.color_power,
        )
    }

    fn hills() -> BiomeParams {
        BiomeParams::new(
            HILLS.name.to_string(),
            HILLS.height_scale,
            HILLS.base_height,
            HILLS.octaves,
            HILLS.persistence,
            HILLS.base_color,
            HILLS.peak_color,
            HILLS.color_power,
        )
    }

    fn desert() -> BiomeParams {
        BiomeParams::new(
            DESERT.name.to_string(),
            DESERT.height_scale,
            DESERT.base_height,
            DESERT.octaves,
            DESERT.persistence,
            DESERT.base_color,
            DESERT.peak_color,
            DESERT.color_power,
        )
    }
}
