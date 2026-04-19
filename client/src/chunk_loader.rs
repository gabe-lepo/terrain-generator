use crate::chunk_batch::{BATCH_SIZE, BatchCoord};
use crate::config::*;
use crate::planet::{HeightBand, PlanetConfig};

use noise::{NoiseFn, Perlin};
use raylib::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{sync::mpsc, task::JoinSet};

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

/// Data for batch of chunks
pub struct BatchData {
    pub coord: BatchCoord,
    pub vertices: Vec<Vector3>,
    pub colors: Vec<Color>,
    pub bbox: BoundingBox,
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
    thread_shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    batch_request_rx: mpsc::UnboundedReceiver<BatchRequest>,
}

impl ChunkLoader {
    /// Create new chunk loader with dedicated runtime
    pub fn new(noise: Perlin, planet: Arc<PlanetConfig>) -> Self {
        let (request_tx, chunk_request_rx) = mpsc::unbounded_channel::<ChunkRequest>();
        let (chunk_data_completed_tx, completed_rx) = mpsc::unbounded_channel::<ChunkData>();
        let (_heightmap_request_tx, heightmap_request_rx) =
            mpsc::unbounded_channel::<HeightmapMeshRequest>();
        let (_thread_shutdown_tx, thread_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
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
            thread_shutdown_rx,
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
    ) {
        let _ = self
            .batch_request_tx
            .send(BatchRequest { coord, heightmaps });
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
        // Return water level outside planet boundary
        let world_size = planet.grid_size as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
        if x < 0.0 || z < 0.0 || x >= world_size || z >= world_size {
            return planet.base_height;
        }

        let seed_offset = (planet.seed.wrapping_mul(2654435761) % 100_000) as f64;
        let continent_offset = (planet.seed.wrapping_mul(1234567891) % 100_000) as f64;

        // Continental masking
        let cx = x as f64 * planet.continent_freq + continent_offset;
        let cz = z as f64 * planet.continent_freq + continent_offset;
        let continent = noise.get([cx, cz]);
        if continent < planet.water_threshold {
            let depth = (continent - planet.water_threshold) * 50.0;
            return (planet.base_height as f64 + depth) as f32;
        }

        // NOTE: This doesnt work well... makes landmasses less-unique
        // Domain warping
        // let warp_freq = planet.freq_scale * 2.0;
        // let warp_x = noise.get([
        //     x as f64 * warp_freq + seed_offset + 1.7,
        //     z as f64 * warp_freq + seed_offset + 9.2,
        // ]);
        // let warp_z = noise.get([
        //     x as f64 * warp_freq + seed_offset + 8.3,
        //     z as f64 * warp_freq + seed_offset + 2.8,
        // ]);

        // Noise detail loop
        let mut total = 0.0_f64;
        let mut amplitude = 1.0_f64;
        let mut frequency = planet.freq_scale;
        let mut max_value = 0.0_f64;

        for _ in 0..planet.octaves {
            let nx = (x as f64) * frequency + seed_offset;
            let nz = (z as f64) * frequency + seed_offset;
            // let nx = (x as f64 + warp_x * planet.warp_strength) * frequency + seed_offset;
            // let nz = (z as f64 + warp_z * planet.warp_strength) * frequency + seed_offset;
            total += noise.get([nx, nz]) * amplitude;
            max_value += amplitude;
            amplitude *= planet.persistence as f64;
            frequency *= planet.lacunarity;
        }

        let normalized = total / max_value;
        // Scale land height by how far above water thresh we are
        let continent_factor = ((continent - planet.water_threshold)
            / (1.0 - planet.water_threshold))
            .clamp(0.0, 1.0)
            .powf(planet.continent_slope);
        let land_height =
            ((normalized + 1.0) / 2.0) * planet.height_scale as f64 * continent_factor;

        (planet.base_height as f64 + land_height) as f32
    }
}

/// Main chunk loader task
async fn chunk_loader_task(
    mut channels: ChunkLoaderChannels,
    noise: Arc<Perlin>,
    planet: Arc<PlanetConfig>,
) {
    let mut tasks: JoinSet<()> = JoinSet::new();
    let send_error_count = Arc::new(AtomicUsize::new(0));
    loop {
        tokio::select! {
            _ = &mut channels.thread_shutdown_rx => {
                // shutdown when regenerated by new seed
                println!("Aborting original loader task due to new seed!");
                tasks.abort_all();
                break;
            }
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
                    let data = build_batch_data(request.coord, &request.heightmaps, &planet);
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

fn build_mesh_data(heightmap: &[Vec<f32>], planet: &PlanetConfig) -> (Vec<Vector3>, Vec<Color>) {
    let grid_size = heightmap.len() - 1;
    let vertex_count = grid_size * grid_size * 6;

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut colors = Vec::with_capacity(vertex_count);

    // Generate vertices
    for z in 0..grid_size {
        for x in 0..grid_size {
            let v00 = Vector3::new(
                x as f32 * TERRAIN_RESOLUTION,
                heightmap[z][x],
                z as f32 * TERRAIN_RESOLUTION,
            );
            let v10 = Vector3::new(
                (x + 1) as f32 * TERRAIN_RESOLUTION,
                heightmap[z][x + 1],
                z as f32 * TERRAIN_RESOLUTION,
            );
            let v01 = Vector3::new(
                x as f32 * TERRAIN_RESOLUTION,
                heightmap[z + 1][x],
                (z + 1) as f32 * TERRAIN_RESOLUTION,
            );
            let v11 = Vector3::new(
                (x + 1) as f32 * TERRAIN_RESOLUTION,
                heightmap[z + 1][x + 1],
                (z + 1) as f32 * TERRAIN_RESOLUTION,
            );

            let c1 = height_to_color((v00.y + v01.y + v10.y) / 3.0, &planet.bands);
            vertices.extend_from_slice(&[v00, v01, v10]);
            colors.extend_from_slice(&[c1, c1, c1]);

            let c2 = height_to_color((v10.y + v01.y + v11.y) / 3.0, &planet.bands);
            vertices.extend_from_slice(&[v10, v01, v11]);
            colors.extend_from_slice(&[c2, c2, c2]);
        }
    }

    (vertices, colors)
}

fn height_to_color(height: f32, bands: &[HeightBand]) -> Color {
    // Find first band whose max_y is >= height
    for i in 0..bands.len() {
        if height <= bands[i].max_y {
            // First band has no blending
            if i == 0 {
                return bands[0].color;
            }

            // Blend between previous and current band
            let prev = &bands[i - 1];
            let curr = &bands[i];
            let band_range = curr.max_y - prev.max_y;
            let t = ((height - prev.max_y) / band_range).clamp(0.0, 1.0);
            return prev.color.lerp(curr.color, t);
        }
    }

    // Above all bands, return top color
    bands.last().map(|b| b.color).unwrap_or(Color::WHITE)
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
) -> BatchData {
    let chunk_world_size = CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
    let (origin_cx, origin_cz) = coord.origin_chunk();
    let origin_world_x = origin_cx as f32 * chunk_world_size;
    let origin_world_z = origin_cz as f32 * chunk_world_size;

    let verts_per_chunk = CHUNK_SIZE * CHUNK_SIZE * 6;
    let mut vertices = Vec::with_capacity(BATCH_SIZE * BATCH_SIZE * verts_per_chunk as usize);
    let mut colors = Vec::with_capacity(BATCH_SIZE * BATCH_SIZE * verts_per_chunk as usize);

    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;

    for (dz, _) in heightmaps.iter().enumerate().take(BATCH_SIZE) {
        for (dx, _) in heightmaps.iter().enumerate().take(BATCH_SIZE) {
            let offset_x = dx as f32 * chunk_world_size;
            let offset_z = dz as f32 * chunk_world_size;
            let heightmap = &heightmaps[dz][dx];

            let (chunk_verts, chunk_colors) = build_mesh_data(heightmap, planet);

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
    }
}
