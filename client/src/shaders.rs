use raylib::ffi;
use raylib::prelude::*;

pub struct ShaderManager {
    fog_shader: Option<Shader>,
    fog_camera_loc: i32,
    fog_color_loc: i32,
    fog_near_loc: i32,
    fog_far_loc: i32,
}

impl ShaderManager {
    pub fn new() -> Self {
        Self {
            fog_shader: None,
            fog_camera_loc: -1,
            fog_color_loc: -1,
            fog_near_loc: -1,
            fog_far_loc: -1,
        }
    }

    pub fn load_shaders(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        self.fog_shader = Self::load_fog_shader(rl, thread);

        // Cache uniform locations if shader loads succseffully
        if let Some(ref shader) = self.fog_shader {
            self.fog_camera_loc = shader.get_shader_location("cameraPosition");
            self.fog_color_loc = shader.get_shader_location("fogColor");
            self.fog_near_loc = shader.get_shader_location("fogNear");
            self.fog_far_loc = shader.get_shader_location("fogFar");

            println!("Shader uniform locations:");
            println!("\tcameraPosition: {}", self.fog_camera_loc);
            println!("\tfogColor: {}", self.fog_color_loc);
            println!("\tfogNear: {}", self.fog_near_loc);
            println!("\tfogFar: {}", self.fog_far_loc);
        } else {
            println!("ERROR: Fog shader failed to load");
        }
    }

    pub fn update_fog_shader(
        &self,
        camera: &Camera3D,
        fog_near: f32,
        fog_far: f32,
        fog_color: Color,
    ) {
        if let Some(ref shader) = self.fog_shader {
            // Set camera position
            // WARN: Unsafe block, FFI not wrapped
            unsafe {
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_camera_loc,
                    [camera.position.x, camera.position.y, camera.position.z].as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3 as i32,
                );

                // Set fog color (normalized 0-1)
                let fog_color_norm = [
                    fog_color.r as f32 / 255.0,
                    fog_color.g as f32 / 255.0,
                    fog_color.b as f32 / 255.0,
                    fog_color.a as f32 / 255.0,
                ];
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_color_loc,
                    fog_color_norm.as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC4 as i32,
                );

                // Set fog distance
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_near_loc,
                    &fog_near as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );

                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    self.fog_far_loc,
                    &fog_far as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );
            }
        }
    }

    pub fn get_fog_shader(&self) -> Option<&Shader> {
        self.fog_shader.as_ref()
    }

    pub fn get_fog_shader_mut(&mut self) -> Option<&mut Shader> {
        self.fog_shader.as_mut()
    }

    // Private
    fn load_fog_shader(rl: &mut RaylibHandle, thread: &RaylibThread) -> Option<Shader> {
        // Use absolute path from manifest directory so it works regardless of CWD
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let vs_path = format!("{}/shaders/fog.vs", manifest_dir);
        let fs_path = format!("{}/shaders/fog.fs", manifest_dir);

        println!("Loading shaders from:");
        println!("  Vertex: {}", vs_path);
        println!("  Fragment: {}", fs_path);

        let shader = rl.load_shader(thread, Some(&vs_path), Some(&fs_path));
        println!("Fog shader loaded (otherwise the underlying FFI would've crashed...)");

        Some(shader)
    }
}
