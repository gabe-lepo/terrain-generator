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

    pub fn update(&mut self, rl: &RaylibHandle, dt: f32) {
        self.handle_mouse_look(rl);
        self.handle_movement(rl, dt);
    }

    pub fn get_camera(&self) -> Camera3D {
        Camera3D::perspective(
            self.position,
            self.position + self.get_forward_vec(),
            Vector3::up(),
            70.0,
        )
    }

    // Private
    fn handle_mouse_look(&mut self, rl: &RaylibHandle) {
        let mouse_delta = rl.get_mouse_delta();
        self.yaw -= mouse_delta.x * MOUSE_SENSITIVITY;
        self.pitch -= mouse_delta.y * MOUSE_SENSITIVITY;
        self.pitch -= self.pitch.clamp(-1.5, 1.5);
    }

    fn handle_movement(&mut self, rl: &RaylibHandle, dt: f32) {
        let forward = self.get_forward_vec();
        let right = self.get_right_vec();

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
        if rl.is_key_down(KeyboardKey::KEY_SPACE) {
            self.position.y += MOVE_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            self.position.y -= MOVE_SPEED * dt;
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

    // TODO: Methods
    // - pub fn handle_input(&mut self, rl: &RaylibHandle, dt: f32) {}
    // - pub fn update_physics(&mut self, dt: f32) {}
    // - pub fn get_camera(&self) -> Camera3D {}
}
