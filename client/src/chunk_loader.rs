use crate::chunk_batch::{BATCH_SIZE, BatchCoord};
use crate::config::*;
use crate::feature_stamp::stamp_contribution;
use crate::planet::{HeightBand, PlanetConfig, height_to_color};
use crate::terrain_shaper::{sample_continent_at, ShapingContext};

use noise::Perlin;
use raylib::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;

/// Request to generate a chunk
pub struct ChunkRequest {
    pub coord: (i32, i32),
    pub heightmap_only: bool,
}

/// Completed chunk data (CPU side only, for GPU upload)
pub struct ChunkData {
    pub coord: (i32, i32),
    pub heightmap: Vec<Vec<f32>>,
}

/// Request struct for heightmap mesh only
pub struct HeightmapMeshRequest {
    pub coord: (i32, i32),
    pub heightmap: Vec<Vec<f32>>,
}

/// Chunk batch request
pub struct BatchRequest {
    pub coord: BatchCoord,
    pub heightmaps: Box<[[Vec<Vec<f32>>; BATCH_SIZE]; BATCH_SIZE]>,
    pub lod: usize,
}

/// Data for batch of chunks
pub struct BatchData {
    pub coord: BatchCoord,
    pub vertices: Vec<Vector3>,
    pub colors: Vec<Color>,
    pub bbox: BoundingBox,
    pub lod: usize,
}

/// Handle for chunk throughput
pub struct ChunkLoader {
    request_tx: mpsc::UnboundedSender<ChunkRequest>,
    completed_rx: mpsc::UnboundedReceiver<ChunkData>,
    batch_request_tx: mpsc::UnboundedSender<BatchRequest>,
    batch_completed_rx: mpsc::UnboundedReceiver<BatchData>,
}

pub struct ChunkLoaderChannels {
    chunk_request_rx: mpsc::UnboundedReceiver<ChunkRequest>,
    heightmap_request_rx: mpsc::UnboundedReceiver<HeightmapMeshRequest>,
    chunk_data_completed_tx: mpsc::UnboundedSender<ChunkData>,
    batch_data_completed_tx: mpsc::UnboundedSender<BatchData>,
    batch_request_rx: mpsc::UnboundedReceiver<BatchRequest>,
}

impl ChunkLoader {
    /// Create new chunk loader with dedicated runtime
    pub fn new(noise: Perlin, planet: Arc<PlanetConfig>) -> Self {
        let (request_tx, chunk_request_rx) = mpsc::unbounded_channel::<ChunkRequest>();
        let (chunk_data_completed_tx, completed_rx) = mpsc::unbounded_channel::<ChunkData>();
        let (_heightmap_request_tx, heightmap_request_rx) =
            mpsc::unbounded_channel::<HeightmapMeshRequest>();
        let (batch_request_tx, batch_request_rx) = mpsc::unbounded_channel::<BatchRequest>();
        let (batch_data_completed_tx, batch_completed_rx) = mpsc::unbounded_channel::<BatchData>();

        // Shared ref counters
        let noise = Arc::new(noise);

        // Pack all these channels before moving to chunk loader
        let chunk_channels = ChunkLoaderChannels {
            chunk_request_rx,
            heightmap_request_rx,
            chunk_data_completed_tx,
            batch_data_completed_tx,
            batch_request_rx,
        };

        // Dedicated tokio runtime in separated thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(CHUNK_LOADER_THREAD_POOLS)
                .thread_name("chunk-loader")
                .build()
                .expect("Failed to create chunk loader runtime");

            rt.block_on(async move {
                chunk_loader_task(chunk_channels, noise, planet).await;
            });
        });

        Self {
            request_tx,
            completed_rx,
            batch_completed_rx,
            batch_request_tx,
        }
    }

    /// Get only heightmap
    pub fn request_heightmap_only(&self, coord: (i32, i32)) {
        let _ = self.request_tx.send(ChunkRequest {
            coord,
            heightmap_only: true,
        });
    }

    /// Request batch to be generated
    pub fn request_batch(
        &self,
        coord: BatchCoord,
        heightmaps: Box<[[Vec<Vec<f32>>; BATCH_SIZE]; BATCH_SIZE]>,
        lod: usize,
    ) {
        let _ = self.batch_request_tx.send(BatchRequest {
            coord,
            heightmaps,
            lod,
        });
    }

    /// Poll for completed chunks, non blocking
    pub fn poll_completed(&mut self) -> Option<ChunkData> {
        self.completed_rx.try_recv().ok()
    }

    pub fn poll_batch_completed(&mut self) -> Option<BatchData> {
        self.batch_completed_rx.try_recv().ok()
    }

    /// NOTE: This is the main terrain "shaping" function
    /// TODO: Caves? How to have multiple y values per x,z
    pub fn get_height(x: f32, z: f32, noise: &Perlin, planet: &PlanetConfig) -> f32 {
        let seed_offset = (planet.seed.wrapping_mul(2654435761) % 100_000) as f64;
        let continent_offset = (planet.seed.wrapping_mul(1234567891) % 100_000) as f64;
        let ctx = ShapingContext::new(noise, planet, seed_offset, continent_offset);

        let world_size = planet.grid_size as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
        if x < 0.0 || z < 0.0 || x >= world_size || z >= world_size {
            return 0.0;
        }

        let xf = x as f64;
        let zf = z as f64;

        let continent = sample_continent_at(
            xf,
            zf,
            planet.continent_freq,
            planet.continent_octaves,
            continent_offset,
            noise,
        );

        let (sx, sz) = if planet.use_domain_warp {
            ShapingContext::domain_warp(xf, zf, &ctx)
        } else {
            (xf, zf)
        };

        let raw = ShapingContext::fbm(sx, sz, &ctx, planet.use_ridged);

        // Ridged FBM already returns [0,1]; plain FBM returns [-1,1] and needs remapping
        let normalized = if planet.use_ridged {
            raw
        } else {
            (raw + 1.0) / 2.0
        };

        let shaped = ShapingContext::apply_plateau_curve(normalized, planet.plateau_strength);

        let redistributed = shaped.powf(planet.redistribution_exponent);

        let eroded = if planet.use_erosion {
            let e = ShapingContext::erosion_mask(sx, sz, &ctx);
            if planet.use_ridged {
                redistributed * (1.0 - e * 0.5)
            } else {
                redistributed * e
            }
        } else {
            redistributed
        };

        // Continent blend: ocean zones suppress height toward land_bias
        let continent_influence = (1.0 - continent) * planet.blend_strength;
        let blended = (eroded * (1.0 - continent_influence) + planet.land_bias).min(1.0);

        // Sea level is Y=0; land above, seafloor below
        const WATER_LEVEL: f64 = 0.38;
        let mut final_height = (blended - WATER_LEVEL) * planet.height_scale as f64;

        // NOTE: Temporarily disabled while tuning base pipeline params
        // let stamp_weight = ((blended - WATER_LEVEL) / (1.0 - WATER_LEVEL)).clamp(0.0, 1.0);
        // for stamp in &planet.stamps {
        //     final_height += stamp_contribution(xf, zf, stamp) * stamp_weight;
        // }

        final_height as f32
    }
}

/// Main chunk loader task
async fn chunk_loader_task(
    mut channels: ChunkLoaderChannels,
    noise: Arc<Perlin>,
    planet: Arc<PlanetConfig>,
) {
    let send_error_count = Arc::new(AtomicUsize::new(0));
    loop {
        tokio::select! {
            Some(request) = channels.chunk_request_rx.recv() => {
                let noise = Arc::clone(&noise);
                let planet = Arc::clone(&planet);
                let completed_tx = channels.chunk_data_completed_tx.clone();

                // Spawn task to generate the chunk
                let send_error_count = Arc::clone(&send_error_count);
                tokio::spawn(async move {
                    let result = std::panic::catch_unwind(|| {
                        generate_chunk_data(request.coord, &noise, &planet, request.heightmap_only)
                    });
                    match result {
                        Ok(chunk_data) => {
                            if let Err(err) = completed_tx.send(chunk_data) {
                                let n = send_error_count.fetch_add(1, Ordering::Relaxed);
                                eprintln!("[{}] Error sending completed chunk data: {:?}", n+1, err);
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Chunk generation panicked for coord: {:?}\n\t{:?}",
                                request.coord, e
                            );
                            std::process::abort();
                        }
                    }
                });
            }
            Some(request) = channels.heightmap_request_rx.recv() => {
                let completed_tx = channels.chunk_data_completed_tx.clone();
                let send_error_count = Arc::clone(&send_error_count);
                tokio::spawn(async move {
                    let data = build_chunk_from_heightmap(request.coord, request.heightmap);
                    if let Err(e) = completed_tx.send(data) {
                        let n = send_error_count.fetch_add(1, Ordering::Relaxed);
                        println!("[{}] Failed completed_tx send: {:?}", n + 1, e);
                    }
                });
            }
            Some(request) = channels.batch_request_rx.recv() => {
                let planet = Arc::clone(&planet);
                let batch_completed_tx = channels.batch_data_completed_tx.clone();
                let send_error_count = Arc::clone(&send_error_count);
                tokio::spawn(async move {
                    let data = build_batch_data(request.coord, &request.heightmaps, &planet, request.lod);
                    if let Err(e) = batch_completed_tx.send(data) {
                        let n = send_error_count.fetch_add(1, Ordering::Relaxed);
                        println!("[{}] Failed batch_completed_tx send: {:?}", n + 1, e);
                    }
                });
            }
            else => break,
        }
    }
}

/// Generate all chunk data, cpu only work here
fn generate_chunk_data(
    coord: (i32, i32),
    noise: &Perlin,
    planet: &PlanetConfig,
    heightmap_only: bool,
) -> ChunkData {
    // Only generate heightmap if requested
    let heightmap = generate_heightmap(coord, noise, planet);

    if heightmap_only {
        return ChunkData { coord, heightmap };
    }

    ChunkData {
        coord,
        heightmap: if GOD_MODE { vec![] } else { heightmap },
    }
}

fn generate_heightmap(coord: (i32, i32), noise: &Perlin, planet: &PlanetConfig) -> Vec<Vec<f32>> {
    let grid_size = CHUNK_SIZE as usize + 1;
    let mut heightmap = Vec::with_capacity(grid_size);

    // Convert coord to world pos
    let chunk_world_x = coord.0 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
    let chunk_world_z = coord.1 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

    for z in 0..grid_size {
        let mut row = Vec::with_capacity(grid_size);
        for x in 0..grid_size {
            let world_x = chunk_world_x + (x as f32 * TERRAIN_RESOLUTION);
            let world_z = chunk_world_z + (z as f32 * TERRAIN_RESOLUTION);
            let height = ChunkLoader::get_height(world_x, world_z, noise, planet);
            row.push(height);
        }
        heightmap.push(row);
    }

    heightmap
}

fn build_mesh_data(
    heightmap: &[Vec<f32>],
    planet: &PlanetConfig,
    step: usize,
) -> (Vec<Vector3>, Vec<Color>) {
    let grid_size = heightmap.len() - 1;
    let vertex_count = grid_size * grid_size * 6;

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut colors = Vec::with_capacity(vertex_count);

    // Generate vertices
    let mut x = 0;
    while x + step < heightmap[0].len() {
        let mut z = 0;
        while z + step < heightmap.len() {
            let v00 = Vector3::new(
                x as f32 * TERRAIN_RESOLUTION,
                heightmap[z][x],
                z as f32 * TERRAIN_RESOLUTION,
            );
            let v10 = Vector3::new(
                (x + step) as f32 * TERRAIN_RESOLUTION,
                heightmap[z][x + step],
                z as f32 * TERRAIN_RESOLUTION,
            );
            let v01 = Vector3::new(
                x as f32 * TERRAIN_RESOLUTION,
                heightmap[z + step][x],
                (z + step) as f32 * TERRAIN_RESOLUTION,
            );
            let v11 = Vector3::new(
                (x + step) as f32 * TERRAIN_RESOLUTION,
                heightmap[z + step][x + step],
                (z + step) as f32 * TERRAIN_RESOLUTION,
            );

            let c1 = height_to_color((v00.y + v01.y + v10.y) / 3.0, &planet.bands);
            vertices.extend_from_slice(&[v00, v01, v10]);
            colors.extend_from_slice(&[c1, c1, c1]);

            let c2 = height_to_color((v10.y + v01.y + v11.y) / 3.0, &planet.bands);
            vertices.extend_from_slice(&[v10, v01, v11]);
            colors.extend_from_slice(&[c2, c2, c2]);

            z += step;
        }
        x += step;
    }

    (vertices, colors)
}


fn build_chunk_from_heightmap(coord: (i32, i32), heightmap: Vec<Vec<f32>>) -> ChunkData {
    ChunkData {
        coord,
        heightmap: if GOD_MODE { vec![] } else { heightmap },
    }
}

pub fn build_batch_data(
    coord: BatchCoord,
    heightmaps: &[[Vec<Vec<f32>>; BATCH_SIZE]; BATCH_SIZE],
    planet: &PlanetConfig,
    lod: usize,
) -> BatchData {
    // LOD0->1 (full)
    // LOD1->2 (half)
    // LOD2->4 (quarter)
    let step = 1 << lod;

    let chunk_world_size = CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
    let (origin_cx, origin_cz) = coord.origin_chunk();
    let origin_world_x = origin_cx as f32 * chunk_world_size;
    let origin_world_z = origin_cz as f32 * chunk_world_size;

    let verts_per_chunk = CHUNK_SIZE * CHUNK_SIZE * 6;
    let mut vertices = Vec::with_capacity(BATCH_SIZE * BATCH_SIZE * verts_per_chunk as usize);
    let mut colors = Vec::with_capacity(BATCH_SIZE * BATCH_SIZE * verts_per_chunk as usize);

    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;

    for (dz, chunk_row) in heightmaps.iter().enumerate().take(BATCH_SIZE) {
        for (dx, heightmap) in chunk_row.iter().enumerate() {
            let offset_x = dx as f32 * chunk_world_size;
            let offset_z = dz as f32 * chunk_world_size;

            let (chunk_verts, chunk_colors) = build_mesh_data(heightmap, planet, step);

            for v in &chunk_verts {
                min_height = min_height.min(v.y);
                max_height = max_height.max(v.y);
                vertices.push(Vector3::new(
                    origin_world_x + offset_x + v.x,
                    v.y,
                    origin_world_z + offset_z + v.z,
                ));
            }

            colors.extend_from_slice(&chunk_colors);
        }
    }

    let batch_world_size = BATCH_SIZE as f32 * chunk_world_size;
    let bbox = BoundingBox::new(
        Vector3::new(origin_world_x, min_height, origin_world_z),
        Vector3::new(
            origin_world_x + batch_world_size,
            max_height,
            origin_world_z + batch_world_size,
        ),
    );

    BatchData {
        coord,
        vertices,
        colors,
        bbox,
        lod,
    }
}
