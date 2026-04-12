use raylib::prelude::*;

// Consts
const MOVE_SPEED: f32 = 5.0;
const GRAVITY_FORCE: f32 = 20.0;
const JUMP_FORCE: f32 = 8.0;
const MOUSE_SENSITIVITY: f32 = 0.003;

pub struct Player {
    position: Vector3,
    velocity: Vector3,
    yaw: f32,
    pitch: f32,
    is_grounded: bool,
}

impl Player {
    pub fn new(position: Vector3) -> Self {
        Self {
            position,
            velocity: Vector3::zero(),
            yaw: 0.0,
            pitch: 0.0,
            is_grounded: false,
        }
    }

    // TODO: Methods
    // - pub fn handle_input(&mut self, rl: &RaylibHandle, dt: f32) {}
    // - pub fn update_physics(&mut self, dt: f32) {}
    // - pub fn get_camera(&self) -> Camera3D {}
}
