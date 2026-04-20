use raylib::prelude::*;

pub fn color_to_f32(color: Color) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

pub fn rl_to_primitive_vec3(vec: Vector3) -> [f32; 3] {
    [vec.x, vec.y, vec.z]
}

pub fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}
