use crate::world::WorldQuery;

use raylib::prelude::*;

// Consts
const MOVE_SPEED: f32 = 5.0;
const GRAVITY_FORCE: f32 = 20.0;
const JUMP_FORCE: f32 = 10.0;
const MOUSE_SENSITIVITY: f32 = 0.003;
const GOD_MODE: bool = false;

pub struct Player {
    pub position: Vector3,
    pub velocity: Vector3,
    pub yaw: f32,
    pub pitch: f32,
    pub is_grounded: bool,
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

    pub fn update(&mut self, rl: &RaylibHandle, world: &impl WorldQuery, dt: f32) {
        self.handle_mouse_look(rl);
        self.handle_movement(rl, dt);
        self.update_physics(rl, world, dt);
    }

    pub fn get_camera(&self) -> Camera3D {
        const EYE_HEIGHT: f32 = 1.8;

        let eye_position = Vector3::new(
            self.position.x,
            self.position.y + EYE_HEIGHT,
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

    fn handle_movement(&mut self, rl: &RaylibHandle, dt: f32) {
        // TODO: Fix slower z,x translation when camera facing very up or very down
        let forward = self.get_forward_vec();
        let right = self.get_right_vec();
        let move_speed = if GOD_MODE {
            MOVE_SPEED * 2.5
        } else {
            MOVE_SPEED
        };

        // Horizontal movements
        if rl.is_key_down(KeyboardKey::KEY_W) {
            self.position = self.position + forward * move_speed * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            self.position = self.position - forward * move_speed * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            self.position = self.position - right * move_speed * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            self.position = self.position + right * move_speed * dt;
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

    fn update_physics(&mut self, rl: &RaylibHandle, world: &impl WorldQuery, dt: f32) {
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
    }
}
