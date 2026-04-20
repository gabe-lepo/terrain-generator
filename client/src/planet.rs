use noise::Perlin;
use raylib::prelude::*;

use crate::{
    config::USE_SINGLE_PLANET,
    feature_stamp::{FeatureStamp, StampKind, generate_stamps},
};

#[derive(Clone)]
pub enum PlanetType {
    Jungle,
    Arctic,
    Desert,
    Volcanic,
    Islands,
    // Ocean,
    // Moon,
    // Wasteland,
    // Plains,
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
    pub sky_color: Color,
    // Terrain shaping
    pub height_scale: f32,
    pub base_height: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f64,
    pub freq_scale: f64,
    pub continent_freq: f64,
    pub water_threshold: f64,
    pub continent_slope: f64,
    // Shaping configs
    pub use_ridged: bool,
    pub use_domain_warp: bool,
    pub warp_strength: f64,
    pub use_erosion: bool,
    pub plateau_strength: f64,
    pub shelf_depth: f64,
    // Feature stamping
    pub stamps: Vec<FeatureStamp>,
}

impl PlanetConfig {
    pub fn new(seed: u64) -> Self {
        let mut config: PlanetConfig;
        if USE_SINGLE_PLANET {
            config = Self::volcanic_planet();
        } else {
            let hashed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            config = match hashed % 5 {
                0 => Self::jungle_planet(),
                1 => Self::arctic_planet(),
                2 => Self::desert_planet(),
                3 => Self::volcanic_planet(),
                _ => Self::islands_planet(),
            };
        }
        config.seed = seed;

        // Feature stamp generation
        let noise = Perlin::new(seed as u32);
        let stamp_kinds = Self::stamp_kinds_for(&config.planet_type);
        config.stamps = generate_stamps(seed, &config, &noise, &stamp_kinds);

        config
    }

    pub fn get_planet_name(&self) -> &str {
        match self.planet_type {
            PlanetType::Jungle => "Jungle",
            PlanetType::Arctic => "Arctic",
            PlanetType::Desert => "Desert",
            PlanetType::Volcanic => "Volcanic",
            PlanetType::Islands => "Islands",
        }
    }

    // Private
    fn stamp_kinds_for(planet_type: &PlanetType) -> Vec<(StampKind, u32)> {
        match planet_type {
            PlanetType::Volcanic => vec![
                (StampKind::Volcano, 150),
                (StampKind::Peak, 250),
                (StampKind::Crater, 100),
                (StampKind::Mesa, 25),
            ],
            PlanetType::Desert => vec![
                (StampKind::Volcano, 150),
                (StampKind::Peak, 250),
                (StampKind::Crater, 100),
                (StampKind::Mesa, 25),
            ],
            PlanetType::Arctic => vec![
                (StampKind::Volcano, 150),
                (StampKind::Peak, 250),
                (StampKind::Crater, 100),
                (StampKind::Mesa, 25),
            ],
            PlanetType::Jungle => vec![
                (StampKind::Volcano, 150),
                (StampKind::Peak, 250),
                (StampKind::Crater, 100),
                (StampKind::Mesa, 25),
            ],
            PlanetType::Islands => vec![
                (StampKind::Volcano, 150),
                (StampKind::Peak, 250),
                (StampKind::Crater, 100),
                (StampKind::Mesa, 25),
            ],
        }
    }
    fn jungle_planet() -> Self {
        Self {
            grid_size: 128,
            seed: 0,
            planet_type: PlanetType::Jungle,
            bands: JUNGLE_BANDS.to_vec(),
            sky_color: Color::new(102, 178, 255, 255),
            height_scale: 90.0,
            base_height: 0.0,
            octaves: 3,
            persistence: 0.55,
            lacunarity: 2.0,
            freq_scale: 0.007,
            continent_freq: 0.0005,
            water_threshold: -0.9,
            continent_slope: 1.2,
            use_ridged: false,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: true,
            plateau_strength: 0.2,
            shelf_depth: 0.0,
            stamps: vec![],
        }
    }

    fn arctic_planet() -> Self {
        Self {
            grid_size: 128,
            seed: 0,
            planet_type: PlanetType::Arctic,
            bands: ARCTIC_BANDS.to_vec(),
            sky_color: Color::new(160, 210, 240, 255),
            height_scale: 250.0,
            base_height: 0.0,
            octaves: 4,
            persistence: 0.38,
            lacunarity: 2.0,
            freq_scale: 0.005,
            continent_freq: 0.0005,
            water_threshold: -0.3,
            continent_slope: 2.5,
            use_ridged: true,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: false,
            plateau_strength: 0.1,
            shelf_depth: 150.0,
            stamps: vec![],
        }
    }

    fn desert_planet() -> Self {
        Self {
            grid_size: 128,
            seed: 0,
            planet_type: PlanetType::Desert,
            bands: DESERT_BANDS.to_vec(),
            sky_color: Color::new(200, 170, 100, 255),
            height_scale: 120.0,
            base_height: 0.0,
            octaves: 2,
            persistence: 0.6,
            lacunarity: 2.0,
            freq_scale: 0.006,
            continent_freq: 0.0003,
            water_threshold: -0.9,
            continent_slope: 0.3,
            use_ridged: false,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: false,
            plateau_strength: 0.7,
            shelf_depth: 0.0,
            stamps: vec![],
        }
    }

    fn volcanic_planet() -> Self {
        Self {
            grid_size: 128,
            seed: 0,
            planet_type: PlanetType::Volcanic,
            bands: VOLCANIC_BANDS.to_vec(),
            sky_color: Color::new(80, 40, 20, 255),
            height_scale: 400.0,
            base_height: 0.0,
            octaves: 6,
            persistence: 0.55,
            lacunarity: 2.2,
            freq_scale: 0.01,
            continent_freq: 0.0005,
            water_threshold: 0.1,
            continent_slope: 0.8,
            use_ridged: true,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: true,
            plateau_strength: 0.15,
            shelf_depth: 0.0,
            stamps: vec![],
        }
    }

    fn islands_planet() -> Self {
        Self {
            grid_size: 128,
            seed: 0,
            planet_type: PlanetType::Islands,
            bands: ISLANDS_BANDS.to_vec(),
            sky_color: Color::new(80, 170, 255, 255),
            height_scale: 40.0,
            base_height: 0.0,
            octaves: 3,
            persistence: 0.35,
            lacunarity: 1.8,
            freq_scale: 0.0025,
            continent_freq: 0.001,
            water_threshold: 0.1,
            continent_slope: 1.5,
            use_ridged: false,
            use_domain_warp: true,
            warp_strength: 300.0,
            use_erosion: false,
            plateau_strength: 0.3,
            shelf_depth: 0.0,
            stamps: vec![],
        }
    }
}

#[rustfmt::skip]
static JUNGLE_BANDS: &[HeightBand] = &[
    HeightBand {max_y: 0.0, color: Color::new(30, 80, 160, 255)},
    HeightBand {max_y: 5.0, color: Color::new(60, 160, 120, 255)},
    HeightBand {max_y: 10.0, color: Color::new(200, 180, 90, 255)},
    HeightBand {max_y: 25.0, color: Color::new(40, 110, 30, 255)},
    HeightBand {max_y: 55.0, color: Color::new(55, 130, 40, 255)},
    HeightBand {max_y: 90.0, color: Color::new(80, 100, 55, 255)},
    HeightBand {max_y: 100.0, color: Color::new(120, 110, 80, 255)},
];

#[rustfmt::skip]
static ARCTIC_BANDS: &[HeightBand] = &[
    HeightBand {max_y: -10.0, color: Color::new(25, 45, 75, 255)},
    HeightBand {max_y: 10.0, color: Color::new(140, 160, 175, 255)},
    HeightBand {max_y: 40.0, color: Color::new(95, 100, 105,255)},
    HeightBand {max_y: 90.0, color: Color::new(185, 195, 205, 255)},
    HeightBand {max_y: 160.0, color: Color::new(200, 208, 215, 255)},
    HeightBand {max_y: 250.0, color: Color::new(215, 218, 218, 255)},
];

#[rustfmt::skip]
static DESERT_BANDS: &[HeightBand] = &[
    HeightBand {max_y: 0.0, color: Color::new(180, 90, 30, 255)},
    HeightBand {max_y: 20.0, color: Color::new(210, 150, 60, 255)},
    HeightBand {max_y: 55.0, color: Color::new(200, 130, 50, 255)},
    HeightBand {max_y: 90.0, color: Color::new(160, 80, 30, 255)},
    HeightBand {max_y: 120.0, color: Color::new(220, 200, 160, 255)},
    HeightBand {max_y: 130.0, color: Color::new(235, 215, 180, 255)},
];

#[rustfmt::skip]
static VOLCANIC_BANDS: &[HeightBand] = &[
    HeightBand {max_y: 0.0, color: Color::new(200, 50, 0, 255)},
    HeightBand {max_y: 30.0, color: Color::new(40, 20, 10, 255)},
    HeightBand {max_y: 100.0, color: Color::new(60, 55, 55, 255)},
    HeightBand {max_y: 220.0, color: Color::new(80, 75, 75, 255)},
    HeightBand {max_y: 350.0, color: Color::new(140, 135, 130, 255)},
    HeightBand {max_y: 400.0, color: Color::new(200, 195, 190, 255)},
];

#[rustfmt::skip]
static ISLANDS_BANDS: &[HeightBand] = &[
    HeightBand {max_y: 0.0, color: Color::new(20, 60, 140, 255)},
    HeightBand {max_y: 2.0, color: Color::new(50, 130, 180, 255)},
    HeightBand {max_y: 4.0, color: Color::new(230, 210, 150, 255)},
    HeightBand {max_y: 18.0, color: Color::new(60, 140, 50, 255)},
    HeightBand {max_y: 32.0, color: Color::new(80, 100, 60, 255)},
    HeightBand {max_y: 42.0, color: Color::new(100, 80, 60, 255)},
];
