use crate::chunk::{Chunk, ChunkCoord, sample_heightmap};
use crate::chunk_loader::ChunkLoader;
use crate::config::{
    CHUNK_SIZE, FAR_CLIP_PLANE_DISTANCE, FOG_FAR_PERCENT, FOG_NEAR_PERCENT, MAX_DISTANCE_BUFFER,
    RENDER_WIREFRAME, VIEW_DISTANCE,
};
use crate::planet::PlanetConfig;
use crate::shaders::ShaderManager;
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct TerrainManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    heightmaps: HashMap<ChunkCoord, Vec<Vec<f32>>>,
    preload_total: usize,
    preload_complete: bool,
    noise: Perlin,
    last_player_chunk: Option<ChunkCoord>,
    last_rendered_count: usize,
    pub planet: Arc<PlanetConfig>,
    chunk_loader: ChunkLoader,
    pending_chunks: HashSet<ChunkCoord>,
    ready: bool,
}

impl TerrainManager {
    pub fn new(seed: u64) -> Self {
        let noise = Perlin::new(seed as u32);
        let planet = Arc::new(PlanetConfig::get_planet_config(seed));
        let chunk_loader = ChunkLoader::new(noise, Arc::clone(&planet));

        let grid = planet.grid_size as i32;
        let preload_total = (grid * grid) as usize;

        for x in 0..grid {
            for z in 0..grid {
                chunk_loader.request_heightmap_only((x, z));
            }
        }

        Self {
            chunks: HashMap::new(),
            heightmaps: HashMap::new(),
            preload_total,
            preload_complete: false,
            noise,
            last_player_chunk: None,
            last_rendered_count: 0,
            planet,
            chunk_loader,
            pending_chunks: HashSet::new(),
            ready: false,
        }
    }

    /// Reinitialize world when we get seed from server
    pub fn reinit_with_seed(&mut self, seed: u64) {
        let noise = Perlin::new(seed as u32);
        let planet = Arc::new(PlanetConfig::get_planet_config(seed));
        let chunk_loader = ChunkLoader::new(noise, Arc::clone(&planet));

        let grid = planet.grid_size as i32;
        let preload_total = (grid * grid) as usize;

        for x in 0..grid {
            for z in 0..grid {
                chunk_loader.request_heightmap_only((x, z));
            }
        }

        self.noise = noise;
        self.planet = planet;
        self.chunk_loader = chunk_loader;
        self.chunks.clear();
        self.pending_chunks.clear();
        self.last_player_chunk = None;
        self.heightmaps = HashMap::new();
        self.preload_total = preload_total;
        self.preload_complete = false;
    }

    /// Update which chunks are loaded based on player pos
    pub fn update(
        &mut self,
        player_pos: Vector3,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        terrain_shader: Option<&Shader>,
    ) {
        // Chunk preloader, block everything else until done
        if !self.preload_complete {
            // Drain all completed heightmap-only chunks
            while let Some(data) = self.chunk_loader.poll_completed() {
                let coord = ChunkCoord::new(data.coord.0, data.coord.1);
                self.heightmaps.insert(coord, data.heightmap);
            }

            if self.heightmaps.len() >= self.preload_total {
                self.preload_complete = true;
                self.ready = true;
            }

            return;
        }

        // Block until seed is available (either server seed or offline default)
        // NOTE: Its very likely the seed is available once the chunks are preloaded
        // might be able to remove this
        if !self.ready {
            return;
        }

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
                let shader = if RENDER_WIREFRAME {
                    None
                } else {
                    terrain_shader
                };
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

        let grid = self.planet.grid_size as i32;
        for dx in -VIEW_DISTANCE..=VIEW_DISTANCE {
            for dz in -VIEW_DISTANCE..=VIEW_DISTANCE {
                let cx = current_chunk.x + dx;
                let cz = current_chunk.z + dz;

                // Clamp to planet boundary
                if cx < 0 || cz < 0 || cx >= grid || cz >= grid {
                    continue;
                }

                let chunk_coord = ChunkCoord::new(cx, cz);
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
            if self.heightmaps.contains_key(&coord) {
                let heightmap = self.heightmaps[&coord].clone();
                self.chunk_loader
                    .request_mesh_from_heightmap((coord.x, coord.z), heightmap);
            } else {
                self.chunk_loader.request_chunk((coord.x, coord.z));
            }
            self.pending_chunks.insert(coord);
        }

        // Unload chunks outside view distance
        self.chunks
            .retain(|coord, _| chunks_to_keep.contains(coord));
        self.pending_chunks
            .retain(|coord| chunks_to_keep.contains(coord));
    }

    /// Preload statuses
    pub fn preload_progress(&self) -> f32 {
        if self.preload_complete {
            return 1.0;
        }
        self.heightmaps.len() as f32 / self.preload_total as f32
    }

    pub fn is_preload_complete(&self) -> bool {
        self.preload_complete
    }

    pub fn render(
        &mut self,
        d: &mut RaylibMode3D<RaylibDrawHandle>,
        camera: &Camera3D,
        shader_manager: &ShaderManager,
        fog_near: f32,
        fog_far: f32,
        fog_color: Color,
        sun_direction: Vector3,
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

    // Private
    fn calculate_height_from_noise(&self, x: f32, z: f32) -> f32 {
        ChunkLoader::get_height(x, z, &self.noise, &self.planet)
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
        let chunk_coord = ChunkCoord::from_world_pos(x, z);
        let grid = self.planet.grid_size as i32;

        // Clamp to world bound
        if chunk_coord.x < 0 || chunk_coord.z < 0 || chunk_coord.x >= grid || chunk_coord.z >= grid
        {
            return self.planet.base_height;
        }

        // If chunk is loaded, used cached heightmap
        // Otherwise calc directly from noise
        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            // Convert world pos to local chunk coord
            let (chunk_world_x, chunk_world_z) = chunk_coord.to_world_pos();
            let local_x = x - chunk_world_x;
            let local_z = z - chunk_world_z;

            chunk.get_height_at(local_x, local_z)
        } else if let Some(heightmap) = self.heightmaps.get(&chunk_coord) {
            let (chunk_world_x, chunk_world_z) = chunk_coord.to_world_pos();
            let local_x = x - chunk_world_x;
            let local_z = z - chunk_world_z;

            sample_heightmap(heightmap, local_x, local_z)
        } else {
            // Chunk not loaded, calc from noise
            self.calculate_height_from_noise(x, z)
        }
    }
}
