use raylib::ffi;
use raylib::prelude::*;

use crate::utils::{color_to_f32, rl_to_primitive_vec3};

pub struct ShaderManager {
    terrain_shader: Option<Shader>,
    // Fog uniforms
    fog_camera_loc: i32,
    fog_color_loc: i32,
    fog_near_loc: i32,
    fog_far_loc: i32,
    // Lighting uniforms
    sun_direction_loc: i32,
    sun_color_loc: i32,
    sun_intensity_loc: i32,
    ambient_strength_loc: i32,
}

impl ShaderManager {
    pub fn new() -> Self {
        Self {
            terrain_shader: None,
            fog_camera_loc: -1,
            fog_color_loc: -1,
            fog_near_loc: -1,
            fog_far_loc: -1,
            sun_direction_loc: -1,
            sun_color_loc: -1,
            sun_intensity_loc: -1,
            ambient_strength_loc: -1,
        }
    }

    pub fn load_shaders(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        self.terrain_shader = Self::load_terrain_shader(rl, thread);

        // Cache uniform locations if shader loads succseffully
        if let Some(ref shader) = self.terrain_shader {
            // Fog uniforms
            self.fog_camera_loc = shader.get_shader_location("cameraPosition");
            self.fog_color_loc = shader.get_shader_location("fogColor");
            self.fog_near_loc = shader.get_shader_location("fogNear");
            self.fog_far_loc = shader.get_shader_location("fogFar");

            // Lighting uniforms
            self.sun_direction_loc = shader.get_shader_location("sunDirection");
            self.sun_color_loc = shader.get_shader_location("sunColor");
            self.sun_intensity_loc = shader.get_shader_location("sunIntensity");
            self.ambient_strength_loc = shader.get_shader_location("ambientStrength");
        } else {
            println!("ERROR: Fog shader failed to load");
        }
    }

    pub fn update_terrain_shader(
        &self,
        camera: &Camera3D,
        fog_near: f32,
        fog_far: f32,
        fog_color: Color,
        sun_direction: Vector3,
        sun_color: Color,
        sun_intensity: f32,
        ambient_strength: f32,
    ) {
        if let Some(ref shader) = self.terrain_shader {
            // Set camera position
            // WARN: Unsafe block, FFI not wrapped
            unsafe {
                // Camera position (VEC3)
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_camera_loc,
                    [camera.position.x, camera.position.y, camera.position.z].as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3 as i32,
                );

                // Fog color (VEC4)
                let fog_color_norm = color_to_f32(fog_color);
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_color_loc,
                    fog_color_norm.as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC4 as i32,
                );

                // Fog near (FLOAT)
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_near_loc,
                    &fog_near as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );

                // Fog far (FLOAT)
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_far_loc,
                    &fog_far as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );

                // Sun direction (VEC3)
                let sun_direction_primitive = rl_to_primitive_vec3(sun_direction);
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.sun_direction_loc,
                    sun_direction_primitive.as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3 as i32,
                );

                // Sun color (VEC3, only rgb, shader ignores alpha)
                let sun_color_norm = color_to_f32(sun_color);
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.sun_color_loc,
                    sun_color_norm.as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3 as i32,
                );

                // Sun intensity (FLOAT)
                let sun_color_norm = color_to_f32(sun_color);
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.sun_intensity_loc,
                    &sun_intensity as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );

                // Ambient strength (FLOAT)
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.ambient_strength_loc,
                    &ambient_strength as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );
            }
        }
    }

    pub fn get_terrain_shader(&self) -> Option<&Shader> {
        self.terrain_shader.as_ref()
    }

    // Private
    fn load_terrain_shader(rl: &mut RaylibHandle, thread: &RaylibThread) -> Option<Shader> {
        // Use absolute path from manifest directory so it works regardless of CWD
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let vs_path = format!("{}/shaders/terrain.vs", manifest_dir);
        let fs_path = format!("{}/shaders/terrain.fs", manifest_dir);

        println!("Loading shaders from:");
        println!("  Vertex: {}", vs_path);
        println!("  Fragment: {}", fs_path);

        let shader = rl.load_shader(thread, Some(&vs_path), Some(&fs_path));
        println!("Terrain shader loaded (otherwise the underlying FFI would've crashed...)");

        Some(shader)
    }
}
