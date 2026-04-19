use crate::chunk::{ChunkCoord, sample_heightmap};
use crate::chunk_batch::{BATCH_SIZE, BatchCoord, ChunkBatch};
use crate::chunk_loader::ChunkLoader;
use crate::config::{
    CHUNK_SIZE, CONNECT, FAR_CLIP_PLANE_DISTANCE, FOG_FAR_PERCENT, FOG_NEAR_PERCENT,
    LOD0_BATCH_RADIUS, LOD1_BATCH_RADIUS, LOD2_BATCH_RADIUS, MAX_DISTANCE_BUFFER, RENDER_WIREFRAME,
    VIEW_DISTANCE,
};
use crate::planet::PlanetConfig;
use crate::player;
use crate::shaders::{FogConfig, ShaderManager, SunConfig};
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct TerrainManager {
    batches: HashMap<BatchCoord, ChunkBatch>,
    upgrading_lod: HashMap<BatchCoord, ChunkBatch>,
    pending_batches: HashMap<BatchCoord, usize>,
    heightmaps: HashMap<ChunkCoord, Vec<Vec<f32>>>,
    preload_total: usize,
    preload_complete: bool,
    noise: Perlin,
    last_player_chunk: Option<ChunkCoord>,
    last_rendered_count: usize,
    pub planet: Arc<PlanetConfig>,
    chunk_loader: ChunkLoader,
    ready: bool,
    initial_mesh_expected_cache: Option<usize>,
}

impl TerrainManager {
    pub fn new() -> Self {
        let seed = if CONNECT { 0 } else { 12345 };
        let noise = Perlin::new(seed as u32);
        let planet = Arc::new(PlanetConfig::new(seed));
        let chunk_loader = ChunkLoader::new(noise, Arc::clone(&planet));

        let grid = planet.grid_size as i32;
        let preload_total = (grid * grid) as usize;

        for x in 0..grid {
            for z in 0..grid {
                chunk_loader.request_heightmap_only((x, z));
            }
        }

        Self {
            batches: HashMap::new(),
            upgrading_lod: HashMap::new(),
            pending_batches: HashMap::new(),
            heightmaps: HashMap::new(),
            preload_total,
            preload_complete: false,
            noise,
            last_player_chunk: None,
            last_rendered_count: 0,
            planet,
            chunk_loader,
            ready: false,
            initial_mesh_expected_cache: None,
        }
    }

    /// Reinitialize world when we get seed from server
    pub fn reinit_with_seed(&mut self, seed: u64) {
        println!("reinit_with_seed called with seed: {}", seed);
        let noise = Perlin::new(seed as u32);
        let planet = Arc::new(PlanetConfig::new(seed));
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
        self.batches.clear();
        self.upgrading_lod.clear();
        self.pending_batches.clear();
        self.last_player_chunk = None;
        self.heightmaps = HashMap::new();
        self.preload_total = preload_total;
        self.preload_complete = false;
        self.ready = false;
        self.initial_mesh_expected_cache = None;
    }

    pub fn update(
        &mut self,
        player_pos: Vector3,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        terrain_shader: Option<&Shader>,
    ) {
        // Collect heightmaps until full planet preload is done
        if !self.preload_complete {
            while let Some(data) = self.chunk_loader.poll_completed() {
                let coord = ChunkCoord::new(data.coord.0, data.coord.1);
                self.heightmaps.insert(coord, data.heightmap);
            }

            if self.heightmaps.len() >= self.preload_total {
                self.preload_complete = true;
                self.ready = true;
                // Batch requests will be submitted on first update pass below
            }

            if !self.preload_complete {
                return;
            }
        }

        // Block until seed is available (either server seed or offline default)
        // NOTE: Its very likely the seed is available once the chunks are preloaded
        // might be able to remove this
        if !self.ready {
            return;
        }

        // Set last player chunk before any expected count calcs
        let current_chunk = ChunkCoord::from_world_pos(player_pos.x, player_pos.z);
        let player_moved = self.last_player_chunk != Some(current_chunk);
        if player_moved {
            self.last_player_chunk = Some(current_chunk);
            self.initial_mesh_expected_cache = None;
        }

        let initial_load_complete =
            self.batches.len() >= (self.initial_mesh_expected() as f32 * 0.9) as usize;
        let upload_cap = if initial_load_complete {
            16
        } else {
            usize::MAX
        };
        let mut uploaded_this_frame = 0;

        while uploaded_this_frame < upload_cap {
            if let Some(batch_data) = self.chunk_loader.poll_batch_completed() {
                let shader = if RENDER_WIREFRAME {
                    None
                } else {
                    terrain_shader
                };
                let batch = ChunkBatch::from_data(batch_data, rl, thread, shader);
                self.pending_batches.remove(&batch.coord);
                self.upgrading_lod.remove(&batch.coord);
                self.batches.insert(batch.coord, batch);
                uploaded_this_frame += 1;
            } else {
                break;
            }
        }

        // Only update batch requests if player moved to a different chunk
        if !player_moved {
            return;
        }

        let grid = self.planet.grid_size as i32;
        let mut batches_to_keep: HashSet<BatchCoord> = HashSet::new();

        for dx in -VIEW_DISTANCE..=VIEW_DISTANCE {
            for dz in -VIEW_DISTANCE..=VIEW_DISTANCE {
                let cx = current_chunk.x + dx;
                let cz = current_chunk.z + dz;

                // Clamp to planet boundary
                if cx < 0 || cz < 0 || cx >= grid || cz >= grid {
                    continue;
                }

                let batch_coord = BatchCoord::from_chunk_coord(cx, cz);
                batches_to_keep.insert(batch_coord);

                let player_batch = BatchCoord::from_chunk_coord(current_chunk.x, current_chunk.z);
                let dbx = (batch_coord.gx - player_batch.gx).abs();
                let dbz = (batch_coord.gz - player_batch.gz).abs();
                let batch_dist = dbx.max(dbz);
                let required_lod = if batch_dist <= LOD0_BATCH_RADIUS {
                    0
                } else if batch_dist <= LOD1_BATCH_RADIUS {
                    1
                } else {
                    2
                };

                // Skip if already rendered at correct lod
                if let Some(batch) = self.batches.get(&batch_coord) {
                    if batch.lod == required_lod {
                        continue;
                    }
                    // LOD needs to be upgraded or downgraded as player moves
                    if let Some(old) = self.batches.remove(&batch_coord) {
                        self.upgrading_lod.insert(batch_coord, old);
                    }
                }

                // Skip if already pending at correct LOD
                if let Some(&pending_lod) = self.pending_batches.get(&batch_coord) {
                    if pending_lod == required_lod {
                        continue;
                    }
                    // Pending at wrong LOD - cancel it
                    self.pending_batches.remove(&batch_coord);
                }

                // Submit batch only when all heightmaps are ready
                let (ocx, ocz) = batch_coord.origin_chunk();
                let all_ready = (0..BATCH_SIZE as i32).all(|ddz| {
                    (0..BATCH_SIZE as i32).all(|ddx| {
                        let hx = ocx + ddx;
                        let hz = ocz + ddz;
                        hx >= 0
                            && hz >= 0
                            && hx < grid
                            && hz < grid
                            && self.heightmaps.contains_key(&ChunkCoord::new(hx, hz))
                    })
                });

                if all_ready {
                    let mut heightmaps: Box<[[Vec<Vec<f32>>; BATCH_SIZE]; BATCH_SIZE]> =
                        Box::new(std::array::from_fn(|_| std::array::from_fn(|_| vec![])));

                    for ddz in 0..BATCH_SIZE {
                        for ddx in 0..BATCH_SIZE {
                            let hx = ocx + ddx as i32;
                            let hz = ocz + ddz as i32;
                            heightmaps[ddz][ddx] =
                                self.heightmaps[&ChunkCoord::new(hx, hz)].clone();
                        }
                    }

                    // Assemble the heightmaps
                    self.chunk_loader
                        .request_batch(batch_coord, heightmaps, required_lod);
                    self.pending_batches.insert(batch_coord, required_lod);
                }
            }
        }

        self.batches
            .retain(|coord, _| batches_to_keep.contains(coord));
        self.pending_batches
            .retain(|coord, _| batches_to_keep.contains(coord));
        self.upgrading_lod
            .retain(|coord, _| batches_to_keep.contains(coord));
    }

    /// Preload statuses
    pub fn preload_progress(&mut self) -> f32 {
        if !self.preload_complete {
            return self.heightmaps.len() as f32 / self.preload_total as f32 * 0.5;
        }
        if self.last_player_chunk.is_none() {
            return 0.5;
        }
        let expected = self.initial_mesh_expected();
        if expected == 0 {
            return 1.0;
        }

        // Pending chunks are in-flight, chunks are uploaded
        // together they represent total progress toward expected
        let done = self.batches.len();
        let in_flight = self.pending_batches.len();
        let mesh_progress = ((done + in_flight) as f32 / expected as f32).min(1.0);
        // println!(
        //     "progress: done={} in-flight={} expected={}",
        //     done, in_flight, expected
        // );
        0.5 + mesh_progress * 0.5
    }

    pub fn is_preload_complete(&mut self) -> bool {
        if !self.preload_complete {
            return false;
        }
        let expected = self.initial_mesh_expected();
        self.batches.len() >= (expected as f32 * 0.9) as usize
    }

    pub fn render(
        &mut self,
        d: &mut RaylibMode3D<RaylibDrawHandle>,
        camera: &Camera3D,
        shader_manager: &ShaderManager,
        fog_config: FogConfig,
        sun_config: SunConfig,
    ) {
        let mut rendered_count = 0;

        // Update shader uniforms once before rendering (shaders already set on chunk materials)
        shader_manager.update_terrain_shader(camera, fog_config, sun_config);

        // Render chunks
        for chunk in self.batches.values() {
            if Self::is_chunk_potentially_visible(camera, chunk) {
                chunk.render(d, RENDER_WIREFRAME);
                rendered_count += 1;
            }
        }

        // Overlap upgraded lod chunks
        for (coord, chunk) in &self.upgrading_lod {
            if !self.batches.contains_key(coord)
                && Self::is_chunk_potentially_visible(camera, chunk)
            {
                chunk.render(d, RENDER_WIREFRAME);
            }
        }

        self.last_rendered_count = rendered_count;
    }

    pub fn chunk_count(&self) -> usize {
        self.batches.len()
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

    fn is_chunk_potentially_visible(camera: &Camera3D, chunk: &ChunkBatch) -> bool {
        // Debugging - kill frustum culling
        // return true;

        let bbox = &chunk.bbox;
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

    /// Need to know initial expected for loading screen
    // WARN: Every time i update/optimize the loading pipeline this
    // logic needs to also be updated!!!
    fn initial_mesh_expected(&mut self) -> usize {
        if let Some(cached) = self.initial_mesh_expected_cache {
            return cached;
        }
        let center = match self.last_player_chunk {
            Some(c) => c,
            None => return 1, // Avoid division by zero before first update
        };
        let grid = self.planet.grid_size as i32;

        let player_batch = BatchCoord::from_chunk_coord(center.x, center.z);
        let radius_chunks = LOD2_BATCH_RADIUS * BATCH_SIZE as i32;
        let mut batch_coords: HashSet<BatchCoord> = HashSet::new();
        for dx in -radius_chunks..=radius_chunks {
            for dz in -radius_chunks..=radius_chunks {
                let cx = center.x + dx;
                let cz = center.z + dz;
                if cx >= 0 && cz >= 0 && cx < grid && cz < grid {
                    let batch_coord = BatchCoord::from_chunk_coord(cx, cz);
                    let dbx = (batch_coord.gx - player_batch.gx).abs();
                    let dbz = (batch_coord.gz - player_batch.gz).abs();
                    if dbx.max(dbz) <= LOD2_BATCH_RADIUS {
                        batch_coords.insert(batch_coord);
                    }
                }
            }
        }
        let result = batch_coords.len();
        self.initial_mesh_expected_cache = Some(result);
        result
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

        if let Some(heightmap) = self.heightmaps.get(&chunk_coord) {
            let (chunk_world_x, chunk_world_z) = chunk_coord.get_world_pos();
            let local_x = x - chunk_world_x;
            let local_z = z - chunk_world_z;
            sample_heightmap(heightmap, local_x, local_z)
        } else {
            // Chunk not loaded, calc from noise
            self.calculate_height_from_noise(x, z)
        }
    }
}
