use crate::world::WorldQuery;

use raylib::prelude::*;

// Consts
const MOVE_SPEED: f32 = 5.0;
const GRAVITY_FORCE: f32 = 20.0;
const JUMP_FORCE: f32 = 10.0;
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
        let forward = self.get_forward_vec();
        let right = self.get_right_vec();

        // Horizontal movements
        if rl.is_key_down(KeyboardKey::KEY_W) {
            self.position = self.position + forward * MOVE_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            self.position = self.position - forward * MOVE_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            self.position = self.position - right * MOVE_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            self.position = self.position + right * MOVE_SPEED * dt;
        }

        // God controls
        // if rl.is_key_down(KeyboardKey::KEY_SPACE) {
        //     self.position.y += MOVE_SPEED * dt;
        // }
        // if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
        //     self.position.y -= MOVE_SPEED * dt;
        // }

        // JUmping
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) && self.is_grounded {
            self.velocity.y = JUMP_FORCE;
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
        // Gravity
        self.velocity.y -= GRAVITY_FORCE * dt;

        // Pos from vel
        self.position.y += self.velocity.y * dt;

        // Get ground height at player pos
        let ground_height = world.get_height_at(self.position.x, self.position.z);

        // Check if hit ground
        if self.position.y <= ground_height {
            self.position.y = ground_height;
            self.velocity.y = 0.0;
            self.is_grounded = true;
        } else {
            self.is_grounded = false;
        }
    }
}
