use raylib::prelude::*;
use shared::Vec3;
use uuid::Uuid;

pub struct RemotePlayer {
    pub id: Uuid,
    pub position: Vec3,
}

impl RemotePlayer {
    pub fn new(id: Uuid, position: Vec3) -> Self {
        Self { id, position }
    }

    pub fn update_position(&mut self, position: Vec3) {
        self.position = position;
    }

    /// Render as a simple cube
    // TODO: Do something better
    pub fn render(&self, d: &mut RaylibMode3D<RaylibDrawHandle>) {
        let pos = Vector3::new(self.position.x, self.position.y + 1.0, self.position.z);

        // Draw simple colorred cube
        d.draw_cube(pos, 1.0, 2.0, 1.0, Color::RED);
        d.draw_cube_wires(pos, 1.0, 2.0, 1.0, Color::DARKRED);
    }
}
