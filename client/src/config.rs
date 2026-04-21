//! Game configuration constants

use raylib::prelude::Color;

// ============================================================================
// LIGHTING & TIME OF DAY
// ============================================================================

pub const TIME_STARTING_HOUR: f32 = SUNRISE_START;
pub const TIME_SPEED_20_MIN: f32 = 300.0;
pub const TIME_SPEED_DEBUG: f32 = 3000.0;

/// Hour thresholds for sun/sky transitions
pub const SUNRISE_START: f32 = 5.0;
pub const SUNRISE_END: f32 = 7.0;
pub const SUNSET_START: f32 = 19.0;
pub const SUNSET_END: f32 = 21.0;

/// Sky colors and tinting
pub const SKY_COLOR_NIGHT: Color = Color::new(3, 3, 8, 255);
pub const SKY_SUNRISE_TINT: Color = Color::new(230, 100, 30, 255);
pub const SKY_SUNRISE_TINT_STRENGTH: f32 = 0.6;
pub const AMBIENT_DAY: f32 = 0.35;
pub const AMBIENT_NIGHT: f32 = 0.08;

/// Sun sizing
pub const SUN_RADIUS: f32 = 1_000.0;
pub const SUN_PLAYER_DISTANCE: f32 = 10_000.0;

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
pub const MOVE_SPEED: f32 = if GOD_MODE { 200.0 } else { 50.0 };

/// Gravity force applied after jumping
pub const GRAVITY_FORCE: f32 = 60.0;

/// Jump force applied when jumping
pub const JUMP_FORCE: f32 = 30.0;

/// Sensitivity of mouse movements
pub const MOUSE_SENSITIVITY: f32 = 0.003;

/// Modifier applied when sprint button held down
pub const SPRINT_MULTIPLIER: f32 = 2.0;

/// Modifier applied when crouch button held down
pub const CROUCH_MULTIPLIER: f32 = 0.5;

/// FOV for camera and frustum culling
pub const PLAYER_FOV_DEGREES: f32 = 90.0;

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

/// Size of each chunk in vertices per side
pub const CHUNK_SIZE: i32 = 16;

/// WARN: BREAKS FOG DISTANCE CALCS IF CHANGED
/// World units per vertex in the heightmap
pub const TERRAIN_RESOLUTION: f32 = 10.0;

/// Planet options
pub const USE_SINGLE_PLANET: bool = false;

// ============================================================================
// RENDERING
// ============================================================================

/// If true, render wireframe only (no shaders, no models)
pub const RENDER_WIREFRAME: bool = false;

/// WARN: Dont change this!!!
/// Fog start/end distances as ratio of max render distance
pub const FOG_NEAR_PERCENT: f32 = 0.3;
pub const FOG_FAR_PERCENT: f32 = 0.4;

/// Minimum world height
pub const WORLD_MIN_Y: f32 = -200.0;

/// Maximum world height
pub const WORLD_MAX_Y: f32 = 2_000.0;

/// Async chunk loader thread pool size
pub const CHUNK_LOADER_THREAD_POOLS: usize = 4;

/// How many chunks to load in each direction from player
pub const VIEW_DISTANCE: i32 = 200;

/// Raylib camera max plane clip distance
pub const FAR_CLIP_PLANE_DISTANCE: f32 =
    (VIEW_DISTANCE as f32) * (CHUNK_SIZE as f32) * TERRAIN_RESOLUTION;

/// Level of detail - Full resolution
pub const LOD0_BATCH_RADIUS: i32 = 4;

/// Level of detail - Half resolution
pub const LOD1_BATCH_RADIUS: i32 = 6;

/// Level of detail - Quarter resolution
pub const LOD2_BATCH_RADIUS: i32 = 7;
