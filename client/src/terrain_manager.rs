use crate::biome::BiomeSystem;
use crate::chunk::{CHUNK_SIZE, Chunk, ChunkCoord, TERRAIN_RESOLUTION};
use crate::shaders::ShaderManager;
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::HashMap;

const VIEW_DISTANCE: i32 = 25;
const RENDER_WIREFRAME: bool = false;
const MAX_DISTANCE_BUFFER: f32 = 1.5;

pub struct TerrainManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    noise: Perlin,
    seed_offset: f64,
    last_player_chunk: Option<ChunkCoord>,
    last_rendered_count: usize,
    biome_system: BiomeSystem,
}

impl TerrainManager {
    pub fn new(seed: u32) -> Self {
        let noise = Perlin::new(seed);
        let seed_offset = seed as f64 * 1000.0;
        let biome_system = BiomeSystem::new(noise, seed_offset);

        Self {
            chunks: HashMap::new(),
            noise,
            seed_offset,
            last_player_chunk: None,
            last_rendered_count: 0,
            biome_system,
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
                    let chunk = Chunk::generate(
                        chunk_coord,
                        &self.noise,
                        self.seed_offset,
                        rl,
                        thread,
                        &self.biome_system,
                    );
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
        shader_manager: &ShaderManager,
        fog_near: f32,
        fog_far: f32,
        fog_color: Color,
    ) {
        let mut rendered_count = 0;

        // Set shader on all chunk models if provided
        if !RENDER_WIREFRAME {
            if let Some(shader) = shader_manager.get_fog_shader() {
                // Set fog shader on each chunks material
                for chunk in self.chunks.values_mut() {
                    let materials = chunk.model.materials_mut();
                    if let Some(material) = materials.get_mut(0) {
                        material.as_mut().shader = shader.as_ref().clone();
                    }
                }

                shader_manager.update_fog_shader(camera, fog_near, fog_far, fog_color);
            }
        }

        // Render chunks
        for chunk in self.chunks.values() {
            if Self::is_chunk_potentially_visible(camera, chunk) {
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

        let fog_near = max_distance * 0.4;
        let fog_far = max_distance * 0.5;

        (fog_near, fog_far)
    }

    // Private
    fn calculate_height_from_noise(&self, x: f32, z: f32) -> f32 {
        use crate::chunk::get_height;
        get_height(x, z, &self.noise, self.seed_offset, &self.biome_system)
    }

    fn is_chunk_potentially_visible(camera: &Camera3D, chunk: &Chunk) -> bool {
        let bbox = &chunk.bounding_box;
        // Camera forward direction
        let forward = Vector3::new(
            camera.target.x - camera.position.x,
            camera.target.y - camera.position.y,
            camera.target.z - camera.position.z,
        )
        .normalized();

        // Vector from camera to bounding box center
        let bbox_center = Vector3::new(
            (bbox.min.x + bbox.max.x) / 2.0,
            (bbox.min.y + bbox.max.y) / 2.0,
            (bbox.min.z + bbox.max.z) / 2.0,
        );

        let to_center = Vector3::new(
            bbox_center.x - camera.position.x,
            bbox_center.y - camera.position.y,
            bbox_center.z - camera.position.z,
        );

        // Dot product, is chunk generally in front of camera?
        let dot = forward.x * to_center.x + forward.y * to_center.y + forward.z * to_center.z;

        // Calc bbox radius (half diagonal)
        let radius = Vector3::new(
            (bbox.max.x - bbox.min.x) / 2.0,
            (bbox.max.y - bbox.min.y) / 2.0,
            (bbox.max.z - bbox.min.z) / 2.0,
        )
        .length();

        // Distance check with radius consideration
        let distance = to_center.length();
        let max_distance = (CHUNK_SIZE as f32 * VIEW_DISTANCE as f32) * MAX_DISTANCE_BUFFER;

        // Chunk is visible if its in front with radius tolerance and within range
        dot > -radius && distance < max_distance + radius
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
