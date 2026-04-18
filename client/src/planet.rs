use raylib::prelude::*;

#[derive(Clone)]
pub enum PlanetType {
    Jungle,
    Arctic,
    Desert,
    Volcanic,
    Ocean,
    // Moon,
    // Wasteland,
    // Plains,
    // Islands,
}

#[derive(Clone)]
pub struct HeightBand {
    pub max_y: f32,
    pub color: Color,
}

#[derive(Clone)]
pub struct PlanetConfig {
    pub grid_size: u32,
    pub seed: u64,
    pub planet_type: PlanetType,
    pub bands: Vec<HeightBand>,
    // Terrain shaping
    pub height_scale: f32,
    pub base_height: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f64,
    pub freq_scale: f64,
    pub continent_freq: f64,
    pub water_threshold: f64,
    pub warp_strength: f64,
    pub continent_slope: f64,
}

impl PlanetConfig {
    pub fn get_planet_config(seed: u64) -> Self {
        let mut config = match seed % 5 {
            // TODO: Create the others and update here
            0 => Self::jungle_planet(),
            1 => Self::jungle_planet(),
            2 => Self::jungle_planet(),
            3 => Self::jungle_planet(),
            _ => Self::jungle_planet(),
        };
        config.seed = seed;
        config
    }

    pub fn get_planet_name(&self) -> &str {
        match self.planet_type {
            PlanetType::Jungle => "Jungle",
            PlanetType::Arctic => "Arctic",
            PlanetType::Desert => "Desert",
            PlanetType::Volcanic => "Volcanic",
            PlanetType::Ocean => "Ocean",
        }
    }

    // Private
    fn jungle_planet() -> Self {
        Self {
            grid_size: 128,
            seed: 0,
            planet_type: PlanetType::Jungle,
            bands: JUNGLE_BANDS.to_vec(),
            height_scale: 200.0,
            base_height: 0.0,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            freq_scale: 0.008,
            continent_freq: 0.0005,
            water_threshold: 0.0,
            warp_strength: 300.0,
            continent_slope: 0.5,
        }
    }
}

// Layers
// - Water
// - Sand/beach
// - "main" (grass, rock, etc)
// - Hill
// - Mountain
// - Mountain cap (ice/snow)
#[rustfmt::skip]
static JUNGLE_BANDS: &[HeightBand] = &[
    HeightBand { max_y: 0.0, color: Color::BLUEVIOLET }, // Water
    HeightBand { max_y: 10.0, color: Color::WHEAT }, // Sand
    HeightBand { max_y: 25.0, color: Color::DARKOLIVEGREEN }, // Jungle floor
    HeightBand { max_y: 100.0, color: Color::ROSYBROWN }, // Hill
    HeightBand { max_y: 200.0, color: Color::BURLYWOOD }, // Mountain
    HeightBand { max_y: 220.0, color: Color::ALICEBLUE }, // Cap
];
