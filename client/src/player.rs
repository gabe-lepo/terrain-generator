use crate::config;
use crate::world::WorldQuery;

use config::{
    CROUCH_MULTIPLIER, GOD_MODE, GRAVITY_FORCE, JUMP_FORCE, MOUSE_SENSITIVITY, MOVE_SPEED,
    SPRINT_MULTIPLIER, WORLD_MAX_Y, WORLD_MIN_Y,
};
use raylib::prelude::*;

pub struct Player {
    pub position: Vector3,
    pub velocity: Vector3,
    pub yaw: f32,
    pub pitch: f32,
    pub is_grounded: bool,
    pub eye_height: f32,
}

impl Player {
    pub fn new(position: Vector3) -> Self {
        Self {
            position,
            velocity: Vector3::zero(),
            yaw: 0.0,
            pitch: 0.0,
            is_grounded: false,
            eye_height: 1.8,
        }
    }

    pub fn update(&mut self, rl: &RaylibHandle, world: &impl WorldQuery, dt: f32) {
        self.handle_mouse_look(rl);
        self.handle_movement(rl, dt, world);
        self.update_physics(world, dt);
    }

    pub fn get_camera(&self) -> Camera3D {
        let eye_position = Vector3::new(
            self.position.x,
            self.position.y + self.eye_height,
            self.position.z,
        );

        Camera3D::perspective(
            eye_position,
            eye_position + self.get_forward_vec(),
            Vector3::up(),
            70.0,
        )
    }

    // Private
    fn handle_mouse_look(&mut self, rl: &RaylibHandle) {
        let mouse_delta = rl.get_mouse_delta();
        self.yaw -= mouse_delta.x * MOUSE_SENSITIVITY;
        self.pitch -= mouse_delta.y * MOUSE_SENSITIVITY;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    fn handle_movement(&mut self, rl: &RaylibHandle, dt: f32, world: &impl WorldQuery) {
        let forward_flat = Vector3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let right = self.get_right_vec();
        let mut move_speed = if GOD_MODE {
            MOVE_SPEED * 2.5
        } else {
            MOVE_SPEED
        };

        // Sprinting and crouching mods
        if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            move_speed *= SPRINT_MULTIPLIER;
        }
        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
            move_speed *= CROUCH_MULTIPLIER;
            self.eye_height = 0.8;
        } else {
            self.eye_height = 1.8;
        }

        // Horizontal movements
        if rl.is_key_down(KeyboardKey::KEY_W) {
            self.position = self.position + forward_flat * move_speed * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            self.position = self.position - forward_flat * move_speed * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            self.position = self.position - right * move_speed * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            self.position = self.position + right * move_speed * dt;
        }

        // ensure not clipping after horizontal movement
        let ground_height = world.get_height_at(self.position.x, self.position.z);
        if self.position.y < ground_height {
            self.position.y = ground_height;
            self.is_grounded = true;
        }

        // God controls
        if GOD_MODE {
            if rl.is_key_down(KeyboardKey::KEY_SPACE) {
                self.position.y += move_speed * dt;
            }
            if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
                self.position.y -= move_speed * dt;
            }
        } else {
            if rl.is_key_pressed(KeyboardKey::KEY_SPACE) && self.is_grounded {
                self.velocity.y = JUMP_FORCE;
            }
        }

        // Clamp to world height min and max
        self.position.y = self.position.y.clamp(WORLD_MIN_Y, WORLD_MAX_Y);
    }

    fn get_forward_vec(&self) -> Vector3 {
        Vector3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        )
    }

    fn get_right_vec(&self) -> Vector3 {
        Vector3::new(
            (self.yaw - std::f32::consts::PI / 2.0).sin(),
            0.0,
            (self.yaw - std::f32::consts::PI / 2.0).cos(),
        )
    }

    fn update_physics(&mut self, world: &impl WorldQuery, dt: f32) {
        if GOD_MODE {
            return;
        }

        const GROUND_TOLERANCE: f32 = 0.2; // Distance to ground to snap
        const GROUND_STICK_SPEED: f32 = 10.0; // How fast to stick to terrain

        // Gravity
        self.velocity.y -= GRAVITY_FORCE * dt;

        // Position from velocity
        self.position.y += self.velocity.y * dt;

        // Get ground height at player pos
        let ground_height = world.get_height_at(self.position.x, self.position.z);

        // Vertical distance to ground
        let distance_to_ground = self.position.y - ground_height;

        // Check if grounded with tolerance
        if distance_to_ground <= GROUND_TOLERANCE {
            self.is_grounded = true;

            // If moving downward or on ground, smoothly stick to terrain
            if self.velocity.y <= 0.0 {
                if distance_to_ground < 0.0 {
                    // Below ground, snap up immediately
                    self.position.y = ground_height;
                } else {
                    // Above ground, but close, smoothly move down
                    let stick_amount = GROUND_STICK_SPEED * dt;
                    if distance_to_ground < stick_amount {
                        self.position.y = ground_height;
                    } else {
                        self.position.y -= stick_amount;
                    }
                }
                self.velocity.y = 0.0;
            }
        } else {
            self.is_grounded = false;
        }

        // Clamp player y to world min and max heights
        self.position.y = self.position.y.clamp(WORLD_MIN_Y, WORLD_MAX_Y);
    }
}
