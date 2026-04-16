use crate::biome::BiomeSystem;
use crate::chunk::{Chunk, ChunkCoord};
use crate::chunk_loader::ChunkLoader;
use crate::config::{
    CHUNK_SIZE, FAR_CLIP_PLANE_DISTANCE, FOG_FAR_PERCENT, FOG_NEAR_PERCENT, MAX_DISTANCE_BUFFER,
    RENDER_WIREFRAME, SEED, TERRAIN_RESOLUTION, VIEW_DISTANCE,
};
use crate::shaders::ShaderManager;
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct TerrainManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    noise: Perlin,
    last_player_chunk: Option<ChunkCoord>,
    last_rendered_count: usize,
    biome_system: Arc<BiomeSystem>,
    chunk_loader: ChunkLoader,
    pending_chunks: HashSet<ChunkCoord>,
}

impl TerrainManager {
    pub fn new() -> Self {
        let noise = Perlin::new(SEED);

        // Create chunk loaded with shared noise and biome sys
        let biome_system = Arc::new(BiomeSystem::new(noise));
        let chunk_loader = ChunkLoader::new(noise, Arc::clone(&biome_system));

        Self {
            chunks: HashMap::new(),
            noise,
            last_player_chunk: None,
            last_rendered_count: 0,
            biome_system,
            chunk_loader,
            pending_chunks: HashSet::new(),
        }
    }

    /// Update which chunks are loaded based on player pos
    pub fn update(
        &mut self,
        player_pos: Vector3,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        fog_shader: Option<&Shader>,
    ) {
        let current_chunk = ChunkCoord::from_world_pos(player_pos.x, player_pos.z);

        // PERF:
        // Poll for completed chunks and upload to gpu
        // - Cap chunk uploads per frame
        // - Unless loading initial chunks, then no effective cap with max usize

        let expected_chunks = ((VIEW_DISTANCE * 2 + 1) * (VIEW_DISTANCE * 2 + 1)) as f32;
        let initial_load_complete = self.chunks.len() >= (expected_chunks * 0.9).round() as usize;
        let upload_cap = if initial_load_complete {
            16
        } else {
            usize::MAX
        };
        let mut uploaded_this_frame = 0;

        while uploaded_this_frame < upload_cap {
            if let Some(chunk_data) = self.chunk_loader.poll_completed() {
                let coord = ChunkCoord::new(chunk_data.coord.0, chunk_data.coord.1);
                let shader = if RENDER_WIREFRAME { None } else { fog_shader };
                let chunk = Chunk::from_data(chunk_data, rl, thread, shader);
                self.chunks.insert(coord, chunk);
                self.pending_chunks.remove(&coord);
                uploaded_this_frame += 1;
            } else {
                break;
            }
        }

        // Only update chunks if player moved to different chunk
        if self.last_player_chunk == Some(current_chunk) {
            return;
        }

        self.last_player_chunk = Some(current_chunk);

        // Determine which chunks should load
        let mut chunks_to_keep = std::collections::HashSet::new();

        // Cap chunk load requests
        let mut new_requests: Vec<(i32, ChunkCoord)> = Vec::new();

        for dx in -VIEW_DISTANCE..=VIEW_DISTANCE {
            for dz in -VIEW_DISTANCE..=VIEW_DISTANCE {
                let chunk_coord = ChunkCoord::new(current_chunk.x + dx, current_chunk.z + dz);
                chunks_to_keep.insert(chunk_coord);

                // Request which chunks should load
                if !self.chunks.contains_key(&chunk_coord)
                    && !self.pending_chunks.contains(&chunk_coord)
                {
                    let dist_sq = dx * dx + dz * dz;
                    new_requests.push((dist_sq, chunk_coord));
                }
            }
        }

        // Sort nearest to player before loading
        new_requests.sort_unstable_by_key(|(d, _)| *d);

        for (_, coord) in new_requests {
            self.chunk_loader.request_chunk((coord.x, coord.z));
            self.pending_chunks.insert(coord);
        }

        // Unload chunks outside view distance
        self.chunks
            .retain(|coord, _| chunks_to_keep.contains(coord));
        self.pending_chunks
            .retain(|coord| chunks_to_keep.contains(coord));
    }

    pub fn render(
        &mut self,
        d: &mut RaylibMode3D<RaylibDrawHandle>,
        camera: &Camera3D,
        shader_manager: &ShaderManager,
        fog_near: f32,
        fog_far: f32,
        fog_color: Color,
        sun_direction: [f32; 3],
        sun_color: Color,
        sun_intensity: f32,
        ambient_strength: f32,
    ) {
        let mut rendered_count = 0;

        // Update shader uniforms once before rendering (shaders already set on chunk materials)
        shader_manager.update_terrain_shader(
            camera,
            fog_near,
            fog_far,
            fog_color,
            sun_direction,
            sun_color,
            sun_intensity,
            ambient_strength,
        );

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

    /// Calculate fog distances based on visible distance
    pub fn get_fog_distances(&self) -> (f32, f32) {
        let visible_distance = FAR_CLIP_PLANE_DISTANCE * 0.5;
        let fog_near = FOG_NEAR_PERCENT * visible_distance;
        let fog_far = FOG_FAR_PERCENT * visible_distance;

        (fog_near, fog_far)
    }

    /// Get biome system biome name
    pub fn get_biome_name_at(&self, x: f32, z: f32) -> String {
        self.biome_system.get_biome_at(x, z).name
    }

    // Private
    fn calculate_height_from_noise(&self, x: f32, z: f32) -> f32 {
        use crate::chunk::get_height;
        get_height(x, z, &self.noise, &self.biome_system)
    }

    fn is_chunk_potentially_visible(camera: &Camera3D, chunk: &Chunk) -> bool {
        // Debugging - kill frustum culling
        // return true;

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

            chunk.get_height_at(local_x, local_z)
        } else {
            // Chunk not loaded, calc from noise
            self.calculate_height_from_noise(x, z)
        }
    }
}
