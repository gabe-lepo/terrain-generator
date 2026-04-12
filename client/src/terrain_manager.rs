use crate::chunk::{CHUNK_SIZE, Chunk, ChunkCoord, TERRAIN_RESOLUTION};
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::HashMap;

const VIEW_DISTANCE: i32 = 20;
const RENDER_WIREFRAME: bool = false;

pub struct TerrainManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    noise: Perlin,
    seed_offset: f64,
    last_player_chunk: Option<ChunkCoord>,
    last_rendered_count: usize,
}

impl TerrainManager {
    pub fn new(seed: u32) -> Self {
        let noise = Perlin::new(seed);
        let seed_offset = seed as f64 * 1000.0;

        Self {
            chunks: HashMap::new(),
            noise,
            seed_offset,
            last_player_chunk: None,
            last_rendered_count: 0,
        }
    }

    /// Update which chunks are loaded based on player pos
    pub fn update(&mut self, player_pos: Vector3, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let current_chunk = ChunkCoord::from_world_pos(player_pos.x, player_pos.z);

        // Only update chunks if player moved to different chunk
        if self.last_player_chunk == Some(current_chunk) {
            return;
        }

        self.last_player_chunk = Some(current_chunk);

        // Determine which chunks should load
        let mut chunks_to_keep = std::collections::HashSet::new();

        for dx in -VIEW_DISTANCE..=VIEW_DISTANCE {
            for dz in -VIEW_DISTANCE..=VIEW_DISTANCE {
                let chunk_coord = ChunkCoord::new(current_chunk.x + dx, current_chunk.z + dz);
                chunks_to_keep.insert(chunk_coord);

                // Load chunk if not already
                if !self.chunks.contains_key(&chunk_coord) {
                    // println!("Loading chunk {:?}", chunk_coord);
                    let chunk =
                        Chunk::generate(chunk_coord, &self.noise, self.seed_offset, rl, thread);
                    self.chunks.insert(chunk_coord, chunk);
                }
            }
        }

        // Unload chunks outside view distance
        self.chunks.retain(|coord, _| {
            let should_keep = chunks_to_keep.contains(coord);
            if !should_keep {
                // println!("Unloading chunk {:?}", coord);
            }
            should_keep
        });
    }

    pub fn render(
        &mut self,
        d: &mut RaylibMode3D<RaylibDrawHandle>,
        camera: &Camera3D,
        fog_shader: Option<&Shader>,
        fog_near: f32,
        fog_far: f32,
        fog_color: Color,
    ) {
        let chunk_size = CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
        let mut rendered_count = 0;

        // Set shader on all chunk models if provided
        if let Some(shader) = fog_shader {
            // Set the fog shader on each chunk's material
            for chunk in self.chunks.values_mut() {
                unsafe {
                    let materials = chunk.model.materials_mut();
                    if let Some(material) = materials.get_mut(0) {
                        material.as_mut().shader = shader.as_ref().clone();
                    }
                }
            }

            // Update shader uniforms
            unsafe {
                use raylib::ffi;

                let camera_loc = shader.get_shader_location("cameraPosition");
                let fog_color_loc = shader.get_shader_location("fogColor");
                let fog_near_loc = shader.get_shader_location("fogNear");
                let fog_far_loc = shader.get_shader_location("fogFar");

                // Set camera position
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    camera_loc,
                    [camera.position.x, camera.position.y, camera.position.z].as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3 as i32,
                );

                // Set fog color
                let fog_color_norm = [
                    fog_color.r as f32 / 255.0,
                    fog_color.g as f32 / 255.0,
                    fog_color.b as f32 / 255.0,
                    fog_color.a as f32 / 255.0,
                ];
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    fog_color_loc,
                    fog_color_norm.as_ptr() as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC4 as i32,
                );

                // Set fog distances
                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    fog_near_loc,
                    &fog_near as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );

                ffi::SetShaderValue(
                    shader.as_ref().clone(),
                    fog_far_loc,
                    &fog_far as *const f32 as *const _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                );
            }
        }

        // Render chunks
        for chunk in self.chunks.values() {
            let (world_x, world_z) = chunk.coord.to_world_pos();
            let chunk_pos = Vector3::new(world_x, 0.0, world_z);

            if Self::is_chunk_potentially_visible(camera, chunk_pos, chunk_size) {
                chunk.render(d, RENDER_WIREFRAME, None);
                rendered_count += 1;
            }
        }

        self.last_rendered_count = rendered_count;
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn rendered_chunk_count(&self) -> usize {
        self.last_rendered_count
    }

    /// Calculate fog distances based on current VIEW_DISTANCE
    pub fn get_fog_distances(&self) -> (f32, f32) {
        // Max render distance in world units
        let max_distance = (VIEW_DISTANCE as f32) * (CHUNK_SIZE as f32) * TERRAIN_RESOLUTION;

        // Start fog at 60% of max distance, full fog at 95%
        let fog_near = max_distance * 0.6;
        let fog_far = max_distance * 0.95;

        (fog_near, fog_far)
    }

    // Private
    fn calculate_height_from_noise(&self, x: f32, z: f32) -> f32 {
        use crate::chunk::get_height;
        get_height(x, z, &self.noise, self.seed_offset)
    }

    fn is_chunk_potentially_visible(
        camera: &Camera3D,
        chunk_pos: Vector3,
        chunk_size: f32,
    ) -> bool {
        // Camera forward direction
        let forward = Vector3::new(
            camera.target.x - camera.position.x,
            camera.target.y - camera.position.y,
            camera.target.z - camera.position.z,
        )
        .normalized();

        // Vector from camera to chunk center
        let chunk_center = Vector3::new(
            chunk_pos.x + chunk_size / 2.0,
            chunk_pos.y,
            chunk_pos.z + chunk_size / 2.0,
        );

        let to_chunk = Vector3::new(
            chunk_center.x - camera.position.x,
            chunk_center.y - camera.position.y,
            chunk_center.z - camera.position.z,
        );

        // Dot product, if negative chunk is behind camera
        let dot = forward.x * to_chunk.x + forward.y * to_chunk.y + forward.z * to_chunk.z;

        // Also check distnace (simple sphere test)
        let distance_sq =
            to_chunk.x * to_chunk.x + to_chunk.y * to_chunk.y + to_chunk.z * to_chunk.z;
        let max_distance = (CHUNK_SIZE as f32 * VIEW_DISTANCE as f32) * 1.5; // Buffer it

        dot > 0.0 && distance_sq < max_distance * max_distance
    }
}

impl WorldQuery for TerrainManager {
    fn get_height_at(&self, x: f32, z: f32) -> f32 {
        // Determine which chunk contains this position
        let chunk_coord = ChunkCoord::from_world_pos(x, z);

        // If chunk is loaded, used cached heightmap
        // Otherwise calc directly from noise
        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            // Convert world pos to local chunk coord
            let (chunk_world_x, chunk_world_z) = chunk_coord.to_world_pos();
            let local_x = x - chunk_world_x;
            let local_z = z - chunk_world_z;

            chunk.get_height_at_local(local_x, local_z)
        } else {
            // Chunk not loaded, calc from noise
            self.calculate_height_from_noise(x, z)
        }
    }
}
