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

impl PlanetType {
    pub fn next(&self) -> Self {
        match self {
            PlanetType::Jungle => PlanetType::Arctic,
            PlanetType::Arctic => PlanetType::Desert,
            PlanetType::Desert => PlanetType::Volcanic,
            PlanetType::Volcanic => PlanetType::Islands,
            PlanetType::Islands => PlanetType::Jungle,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            PlanetType::Jungle => PlanetType::Islands,
            PlanetType::Arctic => PlanetType::Jungle,
            PlanetType::Desert => PlanetType::Arctic,
            PlanetType::Volcanic => PlanetType::Desert,
            PlanetType::Islands => PlanetType::Volcanic,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PlanetType::Jungle => "Jungle",
            PlanetType::Arctic => "Arctic",
            PlanetType::Desert => "Desert",
            PlanetType::Volcanic => "Volcanic",
            PlanetType::Islands => "Islands",
        }
    }
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
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f64,
    pub freq_scale: f64,
    pub continent_freq: f64,
    pub continent_octaves: u32,
    pub water_threshold: f64,
    pub blend_strength: f64,
    pub land_bias: f64,
    pub redistribution_exponent: f64,
    // Shaping configs
    pub use_ridged: bool,
    pub use_domain_warp: bool,
    pub warp_strength: f64,
    pub use_erosion: bool,
    pub plateau_strength: f64,
    // Feature stamping
    pub stamps: Vec<FeatureStamp>,
}

impl PlanetConfig {
    pub fn new(seed: u64) -> Self {
        let mut config: PlanetConfig;
        if USE_SINGLE_PLANET {
            config = Self::islands_planet();
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

    pub fn new_typed(seed: u64, planet_type: PlanetType) -> Self {
        let mut config = match planet_type {
            PlanetType::Jungle => Self::jungle_planet(),
            PlanetType::Arctic => Self::arctic_planet(),
            PlanetType::Desert => Self::desert_planet(),
            PlanetType::Volcanic => Self::volcanic_planet(),
            PlanetType::Islands => Self::islands_planet(),
        };
        config.seed = seed;
        let noise = Perlin::new(seed as u32);
        let stamp_kinds = Self::stamp_kinds_for(&config.planet_type);
        config.stamps = generate_stamps(seed, &config, &noise, &stamp_kinds);
        config
    }

    // Private
    fn stamp_kinds_for(planet_type: &PlanetType) -> Vec<(StampKind, u32)> {
        match planet_type {
            PlanetType::Volcanic => vec![
                (StampKind::Volcano, 200),
                (StampKind::Peak, 150),
                (StampKind::Crater, 80),
                (StampKind::Mesa, 10),
            ],
            PlanetType::Desert => vec![
                (StampKind::Mesa, 80),
                (StampKind::Crater, 60),
                (StampKind::Peak, 100),
                (StampKind::Volcano, 10),
            ],
            PlanetType::Arctic => vec![
                (StampKind::Peak, 300),
                (StampKind::Crater, 50),
                (StampKind::Volcano, 20),
                (StampKind::Mesa, 5),
            ],
            PlanetType::Jungle => vec![
                (StampKind::Peak, 200),
                (StampKind::Mesa, 60),
                (StampKind::Crater, 30),
                (StampKind::Volcano, 10),
            ],
            PlanetType::Islands => vec![
                (StampKind::Volcano, 80),
                (StampKind::Peak, 120),
                (StampKind::Crater, 40),
                (StampKind::Mesa, 5),
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
            octaves: 3,
            persistence: 0.55,
            lacunarity: 2.0,
            freq_scale: 0.007,
            continent_freq: 0.0025,
            continent_octaves: 2,
            water_threshold: 0.05,
            blend_strength: 0.85,
            land_bias: 0.05,
            redistribution_exponent: 1.2,
            use_ridged: false,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: true,
            plateau_strength: 0.2,
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
            octaves: 4,
            persistence: 0.38,
            lacunarity: 2.0,
            freq_scale: 0.005,
            continent_freq: 0.0015,
            continent_octaves: 3,
            water_threshold: 0.35,
            blend_strength: 0.9,
            land_bias: 0.05,
            redistribution_exponent: 1.4,
            use_ridged: true,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: false,
            plateau_strength: 0.1,
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
            octaves: 2,
            persistence: 0.6,
            lacunarity: 2.0,
            freq_scale: 0.006,
            continent_freq: 0.0025,
            continent_octaves: 2,
            water_threshold: 0.05,
            blend_strength: 0.8,
            land_bias: 0.08,
            redistribution_exponent: 0.8,
            use_ridged: false,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: false,
            plateau_strength: 0.7,
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
            octaves: 6,
            persistence: 0.55,
            lacunarity: 2.2,
            freq_scale: 0.01,
            continent_freq: 0.004,
            continent_octaves: 2,
            water_threshold: 0.55,
            blend_strength: 0.95,
            land_bias: 0.04,
            redistribution_exponent: 1.6,
            use_ridged: true,
            use_domain_warp: false,
            warp_strength: 0.0,
            use_erosion: true,
            plateau_strength: 0.15,
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
            octaves: 3,
            persistence: 0.35,
            lacunarity: 1.8,
            freq_scale: 0.0025,
            continent_freq: 0.0009,
            continent_octaves: 3,
            water_threshold: 0.55,
            blend_strength: 0.92,
            land_bias: 0.03,
            redistribution_exponent: 1.3,
            use_ridged: false,
            use_domain_warp: true,
            warp_strength: 300.0,
            use_erosion: false,
            plateau_strength: 0.3,
            stamps: vec![],
        }
    }
}

#[rustfmt::skip]
static JUNGLE_BANDS: &[HeightBand] = &[
    HeightBand {max_y: -20.0, color: Color::new(10,  30, 100, 255)},  // deep ocean
    HeightBand {max_y:   0.0, color: Color::new(30,  80, 160, 255)},  // shallow ocean
    HeightBand {max_y:   3.0, color: Color::new(200, 185, 120, 255)}, // beach
    HeightBand {max_y:   8.0, color: Color::new(80,  150,  55, 255)}, // lowland
    HeightBand {max_y:  30.0, color: Color::new(40,  110,  30, 255)}, // jungle floor
    HeightBand {max_y:  55.0, color: Color::new(55,  130,  40, 255)}, // mid jungle
    HeightBand {max_y:  65.0, color: Color::new(100,  85,  60, 255)}, // peaks
];

#[rustfmt::skip]
static ARCTIC_BANDS: &[HeightBand] = &[
    HeightBand {max_y:  -80.0, color: Color::new( 15,  30,  55, 255)}, // deep cold ocean
    HeightBand {max_y:    0.0, color: Color::new( 55,  80, 110, 255)}, // shallow ice water
    HeightBand {max_y:   15.0, color: Color::new(120, 130, 140, 255)}, // ice shelf
    HeightBand {max_y:   50.0, color: Color::new(160, 168, 175, 255)}, // low snowfield
    HeightBand {max_y:  100.0, color: Color::new(195, 200, 208, 255)}, // mid snowfield
    HeightBand {max_y:  155.0, color: Color::new(220, 222, 225, 255)}, // snow peaks
];

#[rustfmt::skip]
static DESERT_BANDS: &[HeightBand] = &[
    HeightBand {max_y: -30.0, color: Color::new(120,  55,  20, 255)}, // ancient sea basin
    HeightBand {max_y:   0.0, color: Color::new(170,  80,  25, 255)}, // salt flat / low basin
    HeightBand {max_y:  15.0, color: Color::new(210, 140,  55, 255)}, // orange sand
    HeightBand {max_y:  45.0, color: Color::new(195, 155,  80, 255)}, // tan/ochre
    HeightBand {max_y:  70.0, color: Color::new(215, 185, 130, 255)}, // pale dune
    HeightBand {max_y:  80.0, color: Color::new(230, 215, 175, 255)}, // rocky white peaks
];

#[rustfmt::skip]
static VOLCANIC_BANDS: &[HeightBand] = &[
    HeightBand {max_y: -130.0, color: Color::new(200,  60,   0, 255)}, // lava sea
    HeightBand {max_y:    0.0, color: Color::new( 25,  12,   5, 255)}, // dark lava crust
    HeightBand {max_y:   20.0, color: Color::new( 35,  30,  28, 255)}, // charred rock
    HeightBand {max_y:   80.0, color: Color::new( 60,  55,  55, 255)}, // dark grey rock
    HeightBand {max_y:  180.0, color: Color::new( 90,  85,  82, 255)}, // medium grey
    HeightBand {max_y:  250.0, color: Color::new(170, 165, 160, 255)}, // ash peaks
];

#[rustfmt::skip]
static ISLANDS_BANDS: &[HeightBand] = &[
    HeightBand {max_y: -10.0, color: Color::new( 10,  45, 110, 255)}, // deep ocean
    HeightBand {max_y:   0.0, color: Color::new( 35, 110, 170, 255)}, // shallow water
    HeightBand {max_y:   1.5, color: Color::new(225, 205, 145, 255)}, // sandy beach
    HeightBand {max_y:  10.0, color: Color::new( 55, 135,  45, 255)}, // tropical green
    HeightBand {max_y:  20.0, color: Color::new( 70,  95,  50, 255)}, // dense green
    HeightBand {max_y:  28.0, color: Color::new( 95,  75,  55, 255)}, // rocky peak
];

pub fn height_to_color(height: f32, bands: &[HeightBand]) -> Color {
    for i in 0..bands.len() {
        if height <= bands[i].max_y {
            if i == 0 {
                return bands[0].color;
            }
            let prev = &bands[i - 1];
            let curr = &bands[i];
            let band_range = curr.max_y - prev.max_y;
            let t = ((height - prev.max_y) / band_range).clamp(0.0, 1.0);
            return prev.color.lerp(curr.color, t);
        }
    }
    bands.last().map(|b| b.color).unwrap_or(Color::WHITE)
}
