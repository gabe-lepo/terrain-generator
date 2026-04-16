//! Game configuration constants
//!
//! All gameplay and rendering constants in one place for easy tweaking.
//! Just change values here and recompile - no need to hunt through modules.

use raylib::prelude::Color;

// ============================================================================
// LIGHTING & TIME OF DAY
// ============================================================================

pub const TIME_STARTING_HOUR: f32 = 6.0;
pub const TIME_SPEED_10_MIN: f32 = 600.0;
pub const TIME_SPEED_DEBUG: f32 = 6000.0;

pub const SUN_COLOR: Color = Color::new(255, 242, 204, 255);
pub const SUN_MAX_INTENSITY: f32 = 0.8;
pub const AMBIENT_DAY: f32 = 0.35;
pub const AMBIENT_NIGHT: f32 = 0.08;

/// Hour thresholds for sun/sky transitions
pub const SUNRISE_START: f32 = 4.5;
pub const SUNRISE_END: f32 = 7.5;
pub const SUNSET_START: f32 = 16.5;
pub const SUNSET_END: f32 = 19.5;

/// Hour thresholds for sky color transitions
/// (sky blends: night -> sunrise color -> day color | day color -> sunset color -> night)
pub const SKY_SUNRISE_MID: f32 = 6.0;
pub const SKY_DAY_START: f32 = 8.5;
pub const SKY_DAY_END: f32 = 15.5;
pub const SKY_SUNSET_MID: f32 = 18.0;

/// Sky colors at each phase
pub const SKY_COLOR_NIGHT: Color = Color::new(3, 3, 8, 255);
pub const SKY_COLOR_SUNRISE: Color = Color::new(230, 102, 38, 255);
pub const SKY_COLOR_DAY: Color = Color::new(102, 178, 255, 255);

/// Sun direction presets (intentionally private -- use SUN_DIRECTION)
const SUN_DIR_HIGH: [f32; 3] = [0.0, 1.0, 0.0];
const SUN_DIR_ANGLED: [f32; 3] = [0.5, 0.7, 0.3];
const SUN_DIR_LOW: [f32; 3] = [0.8, 0.4, 0.2];

/// Active sun direction
pub const SUN_DIRECTION: [f32; 3] = SUN_DIR_ANGLED;

/// Ambient light floor (0.0 black shadows, 1.0 no shading effect)
pub const AMBIENT_STRENGTH: f32 = 0.15;

// ============================================================================
// GENERAL GAME CLIENT OPTIONS
// ============================================================================

/// Set window width
pub const WINDOW_WIDTH: i32 = 2560;

/// Set window height
pub const WINDOW_HEIGHT: i32 = 1440;

// ============================================================================
// PLAYER OPTIONS
// ============================================================================

/// God mode or not (flying, higher speed, no collision)
pub const GOD_MODE: bool = true;

/// Player move speed
pub const MOVE_SPEED: f32 = if GOD_MODE { 200.0 } else { 15.0 };

/// Gravity force applied after jumping
pub const GRAVITY_FORCE: f32 = 30.0;

/// Jump force applied when jumping
pub const JUMP_FORCE: f32 = 30.0;

/// Sensitivity of mouse movements
pub const MOUSE_SENSITIVITY: f32 = 0.003;

/// Modifier applied when sprint button held down
pub const SPRINT_MULTIPLIER: f32 = 2.0;

/// Modifier applied when crouch button held down
pub const CROUCH_MULTIPLIER: f32 = 0.5;

// ============================================================================
// NETWORK / MULTIPLAYER
// ============================================================================

/// Enable/disable network connection attempts
pub const CONNECT: bool = true;

/// Position updates sent per second
/// Higher = smoother movement, more bandwidth
pub const POSITION_UPDATE_RATE_HZ: f32 = 20.0;

/// Decimal places to round position coordinates
/// Lower = less precision, smaller packets
pub const POSITION_ROUND_DECIMALS: u32 = 1;

// ============================================================================
// TERRAIN GENERATION
// ============================================================================

/// Seed used for Perlin noise and other variation offsets
pub const SEED: u32 = 12345;

/// Size of each chunk in vertices per side (e.g., 16 = 16x16 grid)
/// Smaller = more chunks but less memory per chunk
/// Larger = fewer chunks but more memory per chunk
pub const CHUNK_SIZE: i32 = 16;

/// World units per vertex in the heightmap
/// WARN: BREAKS FOG DISTANCE CALCS IF CHANGED
pub const TERRAIN_RESOLUTION: f32 = 10.0;

/// Base frequency for Perlin noise sampling
/// Lower = larger, smoother terrain features
/// Higher = smaller, more detailed features
pub const NOISE_FREQ: f64 = 0.01;

/// Frequency multiplier between noise octaves
/// Typically 2.0 (each octave doubles frequency)
pub const LACUNARITY: f64 = 2.0;

/// Frequency for biome selection noise
/// Lower = larger biome regions
/// Higher = smaller, more varied biomes
pub const BIOME_FREQ: f64 = 0.0005;

// ============================================================================
// RENDERING
// ============================================================================

/// If true, render wireframe only (no shaders, no models)
pub const RENDER_WIREFRAME: bool = false;

/// Multiplier for frustum culling distance beyond view distance
/// Higher = render farther (lower FPS), Lower = cull more (higher FPS)
pub const MAX_DISTANCE_BUFFER: f32 = 2.0;

/// Fog start/end distances as ratio of max render distance
pub const FOG_NEAR_PERCENT: f32 = 0.3;
pub const FOG_FAR_PERCENT: f32 = 0.4;

/// Fog color
pub const FOG_DEBUG: bool = false;
pub const FOG_COLOR: Color = if FOG_DEBUG {
    Color::RED
} else {
    Color::DEEPSKYBLUE
};

/// Minimum world height
pub const WORLD_MIN_Y: f32 = -100.0;

/// Maximum world height
pub const WORLD_MAX_Y: f32 = 2_000.0;

/// Async chunk loader thread pool size
pub const CHUNK_LOADER_THREAD_POOLS: usize = 4;

/// Raylib camera max plane clip distance
pub const FAR_CLIP_PLANE_DISTANCE: f32 =
    (VIEW_DISTANCE as f32) * (CHUNK_SIZE as f32) * TERRAIN_RESOLUTION;

/// How many chunks to load in each direction from player
pub const VIEW_DISTANCE: i32 = 75;

// ============================================================================
// BIOME DEFINITIONS
// ============================================================================

/// Biome configuration parameters
pub struct BiomeConfig {
    /// Biome name string
    pub name: &'static str,
    /// Maximum height variation (world units)
    pub height_scale: f32,
    /// Base/floor height (world units)
    pub base_height: f32,
    /// Number of noise octaves (more = more detail)
    pub octaves: u32,
    /// How much each octave contributes (higher = rougher)
    pub persistence: f32,
    /// Color at lowest elevation
    pub base_color: Color,
    /// Color at highest elevation
    pub peak_color: Color,
    /// Color gradient curve exponent (< 1.0 = favor peak, > 1.0 = favor base)
    pub color_power: f32,
    /// Noise frequency multiplier (< 1.0 = wider features, > 1.0 = tighter)
    pub freq_scale: f32,
}

impl BiomeConfig {
    pub const fn new(
        name: &'static str,
        height_scale: f32,
        base_height: f32,
        octaves: u32,
        persistence: f32,
        base_color: Color,
        peak_color: Color,
        color_power: f32,
        freq_scale: f32,
    ) -> Self {
        Self {
            name,
            height_scale,
            base_height,
            octaves,
            persistence,
            base_color,
            peak_color,
            color_power,
            freq_scale,
        }
    }
}

/// Flat model color for debugging
pub const USE_FLAT_MODEL_COLOR: bool = false;
pub const FLAT_MODEL_COLOR: Color = Color::WHITE;

/// Extreme mountains, like mountains but more mountain
pub const EXTREME_MOUNTAINS: BiomeConfig = BiomeConfig::new(
    "Extreme Mountains",
    300.0,
    200.0,
    5,
    0.25,
    Color::new(192, 192, 192, 255),
    Color::new(0, 0, 255, 255),
    6.5,
    0.1,
);

/// Mountains biome: tall peaks with gray-to-white gradient
pub const MOUNTAINS: BiomeConfig = BiomeConfig::new(
    "Mountains",
    200.0,
    75.0,
    6,
    0.25,
    Color::DARKBROWN,
    Color::new(255, 255, 255, 255),
    3.5,
    0.5,
);

/// Hills biome: rolling terrain with green gradient
pub const HILLS: BiomeConfig = BiomeConfig::new(
    "Hills",
    75.0,
    20.0,
    2,
    0.25,
    Color::DARKOLIVEGREEN,
    Color::new(100, 255, 100, 255),
    1.0,
    1.0,
);

/// Desert biome: flat terrain with... sand colors
pub const DESERT: BiomeConfig = BiomeConfig::new(
    "Desert",
    20.0,
    0.0,
    1,
    0.025,
    Color::new(200, 180, 100, 255),
    Color::new(220, 200, 130, 255),
    0.3,
    1.0,
);
