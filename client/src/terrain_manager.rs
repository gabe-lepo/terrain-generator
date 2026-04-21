use crate::chunk::{ChunkCoord, sample_heightmap};
use crate::chunk_batch::{BATCH_SIZE, BatchCoord, ChunkBatch};
use crate::chunk_loader::ChunkLoader;
use crate::config::*;
use crate::planet::{PlanetConfig, PlanetType};
use crate::shaders::{FogConfig, ShaderManager, SunConfig};
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct Frustum {
    // [a, b, c, d] per plane (ax+by+cz+d) >= 0 means inside view
    planes: [[f32; 4]; 6],
}

impl Frustum {
    fn from_camera(camera: &Camera3D, aspect_ratio: f32) -> Self {
        let pos = camera.position;
        let fwd = (camera.target - pos).normalized();
        let right = fwd.cross(camera.up).normalized();
        let up = right.cross(fwd).normalized();

        let half_v = (PLAYER_FOV_DEGREES.to_radians() * 0.5).tan();
        let half_h = half_v * aspect_ratio;

        // Each side plane normal points inward.
        // Near/far are planes perpendicular to fwd.
        let near_normal = fwd;
        let far_normal = Vector3::new(-fwd.x, -fwd.y, -fwd.z);

        // Right plane: rotate fwd left by half_h angle -> normal points left (inward)
        // Normal = -right*cos + fwd*sin  ... easier: cross product of edge direction with up
        // Top plane normal: (fwd - up*half_v).cross(right) normalized, pointing inward
        let left_normal = (fwd - right * half_h).cross(up).normalized();
        let right_normal = up.cross(fwd + right * half_h).normalized();
        let bottom_normal = right.cross(fwd - up * half_v).normalized();
        let top_normal = (fwd + up * half_v).cross(right).normalized();

        let make_plane = |n: Vector3| -> [f32; 4] {
            // plane: n.dot(p - pos) >= 0  =>  n.dot(p) - n.dot(pos) >= 0
            // so d = -n.dot(pos)
            let d = -(n.x * pos.x + n.y * pos.y + n.z * pos.z);
            [n.x, n.y, n.z, d]
        };

        // near plane passes through pos + fwd*near_dist, far through pos + fwd*far_dist
        let near_pt = Vector3::new(
            pos.x + fwd.x * 0.01,
            pos.y + fwd.y * 0.01,
            pos.z + fwd.z * 0.01,
        );
        let far_pt = Vector3::new(
            pos.x + fwd.x * FAR_CLIP_PLANE_DISTANCE,
            pos.y + fwd.y * FAR_CLIP_PLANE_DISTANCE,
            pos.z + fwd.z * FAR_CLIP_PLANE_DISTANCE,
        );

        let make_plane_through = |n: Vector3, pt: Vector3| -> [f32; 4] {
            let d = -(n.x * pt.x + n.y * pt.y + n.z * pt.z);
            [n.x, n.y, n.z, d]
        };

        Self {
            planes: [
                make_plane(left_normal),
                make_plane(right_normal),
                make_plane(bottom_normal),
                make_plane(top_normal),
                make_plane_through(near_normal, near_pt),
                make_plane_through(far_normal, far_pt),
            ],
        }
    }

    fn contains_aabb(&self, min: Vector3, max: Vector3) -> bool {
        for plane in &self.planes {
            let [a, b, c, d] = *plane;
            // Positive vertex: pick the corner most in direction of the plane normal
            let px = if a >= 0.0 { max.x } else { min.x };
            let py = if b >= 0.0 { max.y } else { min.y };
            let pz = if c >= 0.0 { max.z } else { min.z };
            if a * px + b * py + c * pz + d < 0.0 {
                return false; // AABB fully outside plane
            }
        }
        true
    }
}

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
    pub fn new_with_seed(seed: u64, planet_type: PlanetType) -> Self {
        let noise = Perlin::new(seed as u32);
        let planet = Arc::new(PlanetConfig::new_typed(seed, planet_type));
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
        // NOTE: Its very likely the seed is available once preload is complete
        // might be able to remove this
        if !self.ready {
            return;
        }

        // Only update if player has moved
        let current_chunk = ChunkCoord::from_world_pos(player_pos.x, player_pos.z);
        let player_moved = self.last_player_chunk != Some(current_chunk);
        if player_moved {
            self.last_player_chunk = Some(current_chunk);
            self.initial_mesh_expected_cache = None;
        }

        // Have we finished the initial mesh generation for expected close-to-player chunks?
        let initial_load_complete =
            self.batches.len() >= (self.initial_mesh_expected() as f32 * 0.9) as usize;

        // Cap chunk uploads per frame
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
        // WARN: This must be checked _after_ the per frame upload cap!
        // Otherwise preload progress is stuck at 100%
        if !player_moved {
            return;
        }

        let grid = self.planet.grid_size as i32;
        let mut batches_to_keep: HashSet<BatchCoord> = HashSet::new();

        // Process chunk batches -- LOD determination,
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

                // PERF:
                // Figure out required LOD given the chunk distance from player
                // Lower LOD at further distances
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

                if let Some(batch) = self.batches.get(&batch_coord) {
                    // Skip if already rendered at correct lod
                    if batch.lod == required_lod {
                        continue;
                    }

                    // Otherwise, LOD needs to be upgraded or downgraded
                    if let Some(old) = self.batches.remove(&batch_coord) {
                        self.upgrading_lod.insert(batch_coord, old);
                    }
                }

                // Skip if already pending at correct LOD
                if let Some(&pending_lod) = self.pending_batches.get(&batch_coord) {
                    if pending_lod == required_lod {
                        continue;
                    }
                    // Pending at wrong LOD - evict it
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

        // Batch queues, pending to be rendered, batches that need LOD change, etc
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
        aspect_ratio: f32,
        shader_manager: &ShaderManager,
        fog_config: FogConfig,
        sun_config: SunConfig,
    ) {
        let frustum = Frustum::from_camera(camera, aspect_ratio);
        let mut rendered_count = 0;

        // Update shader uniforms once before rendering (shaders already set on chunk materials)
        shader_manager.update_terrain_shader(camera, fog_config, sun_config);

        // Render chunks
        for chunk in self.batches.values() {
            if frustum.contains_aabb(chunk.bbox.min, chunk.bbox.max) {
                chunk.render(d, RENDER_WIREFRAME);
                rendered_count += 1;
            }
        }

        // Overlap upgraded lod chunks
        for (coord, chunk) in &self.upgrading_lod {
            if !self.batches.contains_key(coord)
                && frustum.contains_aabb(chunk.bbox.min, chunk.bbox.max)
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
            return 0.0;
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
