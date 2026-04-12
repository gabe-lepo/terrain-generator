pub trait WorldQuery {
    fn get_height_at(&self, x: f32, z: f32) -> f32;
}
