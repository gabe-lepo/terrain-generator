use crate::config::{CHUNK_LOADER_THREAD_POOLS, CHUNK_SIZE, GOD_MODE, TERRAIN_RESOLUTION};
use crate::planet::{HeightBand, PlanetConfig};

use noise::{NoiseFn, Perlin};
use raylib::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::Join;
use tokio::{sync::mpsc, task::JoinSet};

/// Request to generate a chunk
pub struct ChunkRequest {
    pub coord: (i32, i32),
    pub heightmap_only: bool,
}

/// Completed chunk data (CPU side only, for GPU upload)
pub struct ChunkData {
    pub coord: (i32, i32),
    pub vertices: Vec<Vector3>,
    pub indices: Vec<u16>,
    pub colors: Vec<Color>,
    pub bounding_box: BoundingBox,
    pub heightmap: Vec<Vec<f32>>,
    pub normals: Vec<[f32; 3]>,
}

/// Request struct for heightmap mesh only
pub struct HeightmapMeshRequest {
    pub coord: (i32, i32),
    pub heightmap: Vec<Vec<f32>>,
}

/// Handle for chunk throughput
pub struct ChunkLoader {
    request_tx: mpsc::UnboundedSender<ChunkRequest>,
    heightmap_request_tx: mpsc::UnboundedSender<HeightmapMeshRequest>,
    completed_rx: mpsc::UnboundedReceiver<ChunkData>,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl ChunkLoader {
    /// Create new chunk loader with dedicated runtime
    pub fn new(noise: Perlin, planet: Arc<PlanetConfig>) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<ChunkRequest>();
        let (completed_tx, completed_rx) = mpsc::unbounded_channel::<ChunkData>();
        let (heightmap_request_tx, heightmap_request_rx) =
            mpsc::unbounded_channel::<HeightmapMeshRequest>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Shared ref counters
        let noise = Arc::new(noise);

        // Dedicated tokio runtime in separated thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(CHUNK_LOADER_THREAD_POOLS)
                .thread_name("chunk-loader")
                .build()
                .expect("Failed to create chunk loader runtime");

            rt.block_on(async move {
                chunk_loader_task(
                    request_rx,
                    heightmap_request_rx,
                    completed_tx,
                    shutdown_rx,
                    noise,
                    planet,
                )
                .await;
            });
        });

        Self {
            request_tx,
            heightmap_request_tx,
            completed_rx,
            shutdown_tx,
        }
    }

    /// Get only heightmap
    pub fn request_heightmap_only(&self, coord: (i32, i32)) {
        let _ = self.request_tx.send(ChunkRequest {
            coord,
            heightmap_only: true,
        });
    }

    /// Request chunk to be generated
    pub fn request_chunk(&self, coord: (i32, i32)) {
        if let Err(err) = self.request_tx.send(ChunkRequest {
            coord,
            heightmap_only: false,
        }) {
            println!("Error requesting chunk! {:?}", err);
        }
    }

    pub fn request_mesh_from_heightmap(&self, coord: (i32, i32), heightmap: Vec<Vec<f32>>) {
        if let Err(e) = self
            .heightmap_request_tx
            .send(HeightmapMeshRequest { coord, heightmap })
        {
            println!("Failed heightmap_request_tx send: {:?}", e);
        }
    }

    /// Poll for completed chunks, non blocking
    pub fn poll_completed(&mut self) -> Option<ChunkData> {
        self.completed_rx.try_recv().ok()
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
    mut request_rx: mpsc::UnboundedReceiver<ChunkRequest>,
    mut heightmap_request_rx: mpsc::UnboundedReceiver<HeightmapMeshRequest>,
    completed_tx: mpsc::UnboundedSender<ChunkData>,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    noise: Arc<Perlin>,
    planet: Arc<PlanetConfig>,
) {
    let mut tasks: JoinSet<()> = JoinSet::new();
    let send_error_count = Arc::new(AtomicUsize::new(0));
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                // shutdown when regenerated by new seed
                println!("Aborting original loader task due to new seed!");
                tasks.abort_all();
                break;
            }
            Some(request) = request_rx.recv() => {
                let noise = Arc::clone(&noise);
                let planet = Arc::clone(&planet);
                let completed_tx = completed_tx.clone();

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
            Some(request) = heightmap_request_rx.recv() => {
                let planet = Arc::clone(&planet);
                let completed_tx = completed_tx.clone();
                tokio::spawn(async move {
                    let data = build_chunk_from_heightmap(request.coord, request.heightmap, &planet);
                    if let Err(e) = completed_tx.send(data) {
                        println!("Failed completed_tx send: {:?}", e);
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
        return ChunkData {
            coord,
            vertices: vec![],
            indices: vec![],
            colors: vec![],
            normals: vec![],
            bounding_box: calculate_bounding_box(coord, &heightmap),
            heightmap,
        };
    }

    // Build mesh data
    let (vertices, indices, colors) = build_mesh_data(&heightmap, planet);

    // Calc bounding box
    let bounding_box = calculate_bounding_box(coord, &heightmap);

    // Normals
    let normals = compute_normals(&heightmap, coord, noise, planet);

    ChunkData {
        coord,
        vertices,
        indices,
        colors,
        bounding_box,
        heightmap: if GOD_MODE { vec![] } else { heightmap },
        normals,
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
    heightmap: &Vec<Vec<f32>>,
    planet: &PlanetConfig,
) -> (Vec<Vector3>, Vec<u16>, Vec<Color>) {
    let grid_size = heightmap.len();
    let vertex_count = grid_size * grid_size;
    let triangle_count = (grid_size - 1) * (grid_size - 1) * 2;

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(triangle_count * 3);
    let mut colors = Vec::with_capacity(vertex_count);

    // Generate vertices
    for z in 0..grid_size {
        for x in 0..grid_size {
            let local_x = x as f32 * TERRAIN_RESOLUTION;
            let local_z = z as f32 * TERRAIN_RESOLUTION;
            let height = heightmap[z][x];

            vertices.push(Vector3::new(local_x, height, local_z));
        }
    }

    // Generate triangle indices
    for z in 0..(grid_size - 1) {
        for x in 0..(grid_size - 1) {
            let top_left = (z * grid_size + x) as u16;
            let top_right = top_left + 1;
            let bottom_left = ((z + 1) * grid_size + x) as u16;
            let bottom_right = bottom_left + 1;

            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    for z in 0..grid_size {
        for x in 0..grid_size {
            let height = heightmap[z][x];
            colors.push(height_to_color(height, &planet.bands));
        }
    }

    (vertices, indices, colors)
}

fn calculate_bounding_box(coord: (i32, i32), heightmap: &Vec<Vec<f32>>) -> BoundingBox {
    let world_x = coord.0 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
    let world_z = coord.1 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;

    for row in heightmap {
        for &height in row {
            min_height = min_height.min(height);
            max_height = max_height.max(height);
        }
    }

    let chunk_size = CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

    BoundingBox::new(
        Vector3::new(world_x, min_height, world_z),
        Vector3::new(world_x + chunk_size, max_height, world_z + chunk_size),
    )
}

fn compute_normals(
    heightmap: &[Vec<f32>],
    coord: (i32, i32),
    noise: &Perlin,
    planet: &PlanetConfig,
) -> Vec<[f32; 3]> {
    let grid_size = heightmap.len();
    let mut normals = Vec::with_capacity(grid_size * grid_size);
    let last = grid_size - 1;

    let chunk_world_x = coord.0 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
    let chunk_world_z = coord.1 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

    for z in 0..grid_size {
        for x in 0..grid_size {
            let h_right = if x < last {
                heightmap[z][x + 1]
            } else {
                let wx = chunk_world_x + (x + 1) as f32 * TERRAIN_RESOLUTION;
                let wz = chunk_world_z + z as f32 * TERRAIN_RESOLUTION;
                ChunkLoader::get_height(wx, wz, noise, planet)
            };

            let h_down = if z < last {
                heightmap[z + 1][x]
            } else {
                let wx = chunk_world_x + x as f32 * TERRAIN_RESOLUTION;
                let wz = chunk_world_z + (z + 1) as f32 * TERRAIN_RESOLUTION;
                ChunkLoader::get_height(wx, wz, noise, planet)
            };

            let nx = heightmap[z][x] - h_right;
            let ny = TERRAIN_RESOLUTION;
            let nz = heightmap[z][x] - h_down;

            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            normals.push([nx / len, ny / len, nz / len]);
        }
    }

    normals
}

fn height_to_color(height: f32, bands: &Vec<HeightBand>) -> Color {
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

fn build_chunk_from_heightmap(
    coord: (i32, i32),
    heightmap: Vec<Vec<f32>>,
    planet: &PlanetConfig,
) -> ChunkData {
    let (vertices, indices, colors) = build_mesh_data(&heightmap, planet);
    let bbox = calculate_bounding_box(coord, &heightmap);
    let normals = vec![];

    ChunkData {
        coord,
        vertices,
        indices,
        colors,
        bounding_box: bbox,
        heightmap: if GOD_MODE { vec![] } else { heightmap },
        normals,
    }
}
