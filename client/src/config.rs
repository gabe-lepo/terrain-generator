//! Game configuration constants
//!
//! All gameplay and rendering constants in one place for easy tweaking.
//! Just change values here and recompile - no need to hunt through modules.

use raylib::prelude::Color;

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
pub const JUMP_FORCE: f32 = 15.0;

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
pub const CONNECT: bool = false;

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
pub const SEED: u32 = 111111;

/// Size of each chunk in vertices per side (e.g., 16 = 16x16 grid)
/// Smaller = more chunks but less memory per chunk
/// Larger = fewer chunks but more memory per chunk
pub const CHUNK_SIZE: i32 = 16;

/// World units per vertex in the heightmap
/// Larger = more spread out terrain, fewer vertices
/// Smaller = more detailed terrain, more vertices
pub const TERRAIN_RESOLUTION: f32 = 2.5;

/// How many chunks to load in each direction from player
/// Total chunks = (VIEW_DISTANCE * 2 + 1)^2
/// 25 = 51x51 grid = 2,601 chunks
pub const VIEW_DISTANCE: i32 = 25;

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

/// If true, render wireframe only (no fog)
/// If false, render solid with fog shader
pub const RENDER_WIREFRAME: bool = false;

/// Multiplier for frustum culling distance beyond view distance
/// Higher = render farther (lower FPS), Lower = cull more (higher FPS)
pub const MAX_DISTANCE_BUFFER: f32 = 2.0;

/// Fog start distance as percentage of max render distance (0.0 - 1.0)
pub const FOG_NEAR_PERCENT: f32 = 0.6;

/// Fog full opacity distance as percentage of max render distance (0.0 - 1.0)
pub const FOG_FAR_PERCENT: f32 = 0.8;

/// Minimum world height
pub const WORLD_MIN_Y: f32 = -100.0;

/// Maximum world height
pub const WORLD_MAX_Y: f32 = 1_000.0;

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

/// Mountains biome: tall peaks with gray-to-white gradient
pub const MOUNTAINS: BiomeConfig = BiomeConfig::new(
    "Mountains",
    200.0,
    40.0,
    6,
    0.5,
    Color::new(50, 50, 50, 255),
    Color::new(255, 255, 255, 255),
    3.5,
    0.5,
);

/// Plains biome: flat terrain with tan/wheat colors
pub const PLAINS: BiomeConfig = BiomeConfig::new(
    "Plains",
    20.0,
    0.0,
    1,
    0.5,
    Color::new(200, 180, 100, 255),
    Color::new(220, 200, 130, 255),
    0.3,
    1.0,
);

/// Hills biome: rolling terrain with green gradient
pub const HILLS: BiomeConfig = BiomeConfig::new(
    "Hills",
    75.0,
    10.0,
    2,
    0.5,
    Color::new(0, 150, 0, 255),
    Color::new(100, 255, 100, 255),
    1.0,
    1.0,
);
