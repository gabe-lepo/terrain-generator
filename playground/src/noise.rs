use crate::{config, utils::normalize_oneone};
use noise::{NoiseFn, Perlin};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

pub struct RedistributionParams {
    pub exponent: f64,
    pub elevation_dependent: bool,
    pub use_smoothstep: bool,
}

impl RedistributionParams {
    pub fn new() -> Self {
        Self {
            exponent: 1.0,
            elevation_dependent: false,
            use_smoothstep: false,
        }
    }
}

fn apply_redistribution(value: f64, params: &RedistributionParams) -> f64 {
    if params.use_smoothstep {
        let edge = ((1.0 - params.exponent / 4.0) * 0.4).clamp(0.0, 0.45);
        smoothstep(edge, 1.0 - edge, value)
    } else if params.elevation_dependent {
        value.powf(1.0 + (params.exponent - 1.0) * value)
    } else {
        value.powf(params.exponent)
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub struct ContinentParams {
    pub frequency: f64,
    pub octaves: u32,
    pub blend_strength: f64,
    pub land_bias: f64,
}

impl ContinentParams {
    pub fn new() -> Self {
        Self {
            frequency: 0.003,
            octaves: 2,
            blend_strength: 0.8,
            land_bias: 0.1,
        }
    }
}

pub struct NoiseParams {
    pub frequency: f64,
    pub octaves: u32,
    pub persistence: f64,
    pub lacunarity: f64,
}

impl NoiseParams {
    pub fn new() -> Self {
        Self {
            frequency: 0.05,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
        }
    }
}

pub enum BaseSignal {
    Random,
    Perlin,
    Fbm,
}

pub struct MoistureParams {
    pub frequency: f64,
    pub offset: f64,
}

impl MoistureParams {
    pub fn new() -> Self {
        Self {
            frequency: 0.005,
            offset: 0.0,
        }
    }
}

pub struct NoiseConfig {
    pub base_signal: BaseSignal,
    pub use_continent_mask: bool,
    pub use_redistribution: bool,
    pub use_moisture: bool,
    pub do_lerp: bool,
    pub noise_params: NoiseParams,
    pub continent_params: ContinentParams,
    pub redistribution_params: RedistributionParams,
    pub moisture_params: MoistureParams,
}

impl NoiseConfig {
    pub fn new(
        noise_params: NoiseParams,
        continent_params: ContinentParams,
        redistribution_params: RedistributionParams,
        moisture_params: MoistureParams,
    ) -> Self {
        Self {
            base_signal: BaseSignal::Random,
            use_continent_mask: false,
            use_redistribution: false,
            use_moisture: false,
            do_lerp: false,
            noise_params,
            continent_params,
            redistribution_params,
            moisture_params,
        }
    }
}

pub fn sample_fbm(perlin: &Perlin, x: f64, y: f64, config: &NoiseConfig) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = config.noise_params.frequency;
    let mut max_value = 0.0;

    for _ in 0..config.noise_params.octaves {
        value += perlin.get([x * frequency, y * frequency]) * amplitude;
        max_value += amplitude;
        amplitude *= config.noise_params.persistence;
        frequency *= config.noise_params.lacunarity;
    }

    (value / max_value + 1.0) / 2.0
}

pub fn sample_perlin(perlin: &Perlin, x: f64, y: f64, config: &NoiseConfig) -> f64 {
    normalize_oneone(perlin.get([
        x * config.noise_params.frequency,
        y * config.noise_params.frequency,
    ]))
}

pub fn sample_continent_mask(perlin: &Perlin, x: f64, y: f64, params: &ContinentParams) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = params.frequency;
    let mut max_value = 0.0;

    for _ in 0..params.octaves {
        value += perlin.get([x * frequency, y * frequency]) * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    (value / max_value + 1.0) / 2.0
}

fn sample_moisture(perlin: &Perlin, x: f64, y: f64, params: &MoistureParams) -> f64 {
    let raw = normalize_oneone(perlin.get([x * params.frequency, y * params.frequency]));
    (raw + params.offset).clamp(0.0, 1.0)
}

pub fn generate_map(config: &NoiseConfig, seed: u64, grid_size: usize) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let perlin = Perlin::new(seed as u32);
    let continent_perlin = Perlin::new(seed.wrapping_add(1) as u32);
    let moisture_perlin = Perlin::new(seed.wrapping_add(2) as u32);
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut heightmap = Vec::with_capacity(grid_size);
    let mut moisturemap = Vec::with_capacity(grid_size);

    for x in 0..grid_size {
        let mut height_row = Vec::with_capacity(grid_size);
        let mut moisture_row = Vec::with_capacity(grid_size);
        for y in 0..grid_size {
            let value = match config.base_signal {
                BaseSignal::Random => rng.random_range(0.0..=1.0),
                BaseSignal::Perlin => sample_perlin(&perlin, x as f64, y as f64, config),
                BaseSignal::Fbm => sample_fbm(&perlin, x as f64, y as f64, config),
            };
            let value = if config.use_redistribution {
                apply_redistribution(value, &config.redistribution_params)
            } else {
                value
            };
            let value = if config.use_continent_mask {
                let mask = sample_continent_mask(
                    &continent_perlin,
                    x as f64,
                    y as f64,
                    &config.continent_params,
                );
                // mask > 0.5 → land zone: value passes through largely unchanged
                // mask < 0.5 → ocean zone: value scaled toward 0
                // blend_strength controls how aggressively ocean zones are suppressed
                let continent_influence = (1.0 - mask) * config.continent_params.blend_strength;
                (value * (1.0 - continent_influence) + config.continent_params.land_bias).min(1.0)
            } else {
                value
            };
            height_row.push(value);
            moisture_row.push(sample_moisture(&moisture_perlin, x as f64, y as f64, &config.moisture_params));
        }
        heightmap.push(height_row);
        moisturemap.push(moisture_row);
    }

    (heightmap, moisturemap)
}
