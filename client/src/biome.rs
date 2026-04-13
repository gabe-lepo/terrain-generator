use noise::{NoiseFn, Perlin};

pub struct BiomeParams {
    pub height_scale: f32,
    pub base_height: f32,
    pub octaves: u32,
    pub persistence: f32,
}

impl BiomeParams {
    pub fn new(height_scale: f32, base_height: f32, octaves: u32, persistence: f32) -> Self {
        Self {
            height_scale,
            base_height,
            octaves,
            persistence,
        }
    }
}

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
        const BIOME_FREQ: f64 = 0.0005; // Lower the larger the region

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
        if noise_value < -0.3 {
            let t = ((noise_value + 1.0) / 0.7) as f32;
            Self::lerp_biomes(&plains, &mountains, t)
        } else if noise_value < 0.3 {
            let t = ((noise_value + 0.3) / 0.6) as f32;
            Self::lerp_biomes(&mountains, &hills, t)
        } else {
            let t = ((noise_value - 0.3) / 0.7) as f32;
            Self::lerp_biomes(&hills, &plains, t)
        }
    }

    /// Lerp between 2 biome param sets
    fn lerp_biomes(a: &BiomeParams, b: &BiomeParams, t: f32) -> BiomeParams {
        BiomeParams {
            height_scale: Self::lerp(a.height_scale, b.height_scale, t),
            base_height: Self::lerp(a.base_height, b.base_height, t),
            octaves: Self::lerp(a.octaves as f32, b.octaves as f32, t).round() as u32,
            persistence: Self::lerp(a.persistence, b.persistence, t),
        }
    }

    /// LERP helper
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    // Define biome presets as assc funcs
    fn mountains() -> BiomeParams {
        BiomeParams::new(200.0, 40.0, 6, 0.5)
    }

    fn plains() -> BiomeParams {
        BiomeParams::new(40.0, 0.0, 2, 0.5)
    }

    fn hills() -> BiomeParams {
        BiomeParams::new(100.0, 0.0, 4, 0.5)
    }
}
