use crate::biome::BiomeSystem;
use crate::config::{
    CHUNK_LOADER_THREAD_POOLS, CHUNK_SIZE, GOD_MODE, LACUNARITY, NOISE_FREQ, SEED,
    TERRAIN_RESOLUTION,
};
use noise::{NoiseFn, Perlin};
use raylib::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Request to generate a chunk
pub struct ChunkRequest {
    pub coord: (i32, i32),
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

/// Handle for chunk throughput
pub struct ChunkLoader {
    request_tx: mpsc::UnboundedSender<ChunkRequest>,
    completed_rx: mpsc::UnboundedReceiver<ChunkData>,
}

impl ChunkLoader {
    /// Create new chunk loader with dedicated runtime
    pub fn new(noise: Perlin, biome_system: Arc<BiomeSystem>) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<ChunkRequest>();
        let (completed_tx, completed_rx) = mpsc::unbounded_channel::<ChunkData>();

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
                chunk_loader_task(request_rx, completed_tx, noise, biome_system).await;
            });
        });

        Self {
            request_tx,
            completed_rx,
        }
    }

    /// Request chunk to be generated
    pub fn request_chunk(&self, coord: (i32, i32)) {
        if let Err(err) = self.request_tx.send(ChunkRequest { coord }) {
            println!("Error requesting chunk! {:?}", err);
        }
    }

    /// Poll for completed chunks, non blocking
    pub fn poll_completed(&mut self) -> Option<ChunkData> {
        self.completed_rx.try_recv().ok()
    }
}

/// Main chunk loader task
async fn chunk_loader_task(
    mut request_rx: mpsc::UnboundedReceiver<ChunkRequest>,
    completed_tx: mpsc::UnboundedSender<ChunkData>,
    noise: Arc<Perlin>,
    biome_system: Arc<BiomeSystem>,
) {
    while let Some(request) = request_rx.recv().await {
        let noise = Arc::clone(&noise);
        let biome_system = Arc::clone(&biome_system);
        let completed_tx = completed_tx.clone();

        // Spawn task to generate the chunk
        tokio::spawn(async move {
            // TODO: Generate heightmap, vertices, indices, colors, bbox
            // This is where we move the chunk gen logic

            let chunk_data = generate_chunk_data(request.coord, &noise, &biome_system);
            if let Err(err) = completed_tx.send(chunk_data) {
                println!("Error sending completed chunk data! {:?}", err);
            }
        });
    }
}

/// Generate all chunk data, cpu only work here
fn generate_chunk_data(coord: (i32, i32), noise: &Perlin, biome_system: &BiomeSystem) -> ChunkData {
    // ALWAYS generate heightmap, only return it if not god mode
    let heightmap = generate_heightmap(coord, noise, biome_system);

    // Build mesh data
    let (vertices, indices, colors) = build_mesh_data(coord, &heightmap, biome_system);

    // Calc bounding box
    let bounding_box = calculate_bounding_box(coord, &heightmap);

    // Normals
    let normals = compute_normals(&heightmap);

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

fn generate_heightmap(
    coord: (i32, i32),
    noise: &Perlin,
    biome_system: &BiomeSystem,
) -> Vec<Vec<f32>> {
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
            let height = get_height(world_x, world_z, noise, biome_system);
            row.push(height);
        }
        heightmap.push(row);
    }

    heightmap
}

fn get_height(x: f32, z: f32, noise: &Perlin, biome_system: &BiomeSystem) -> f32 {
    let seed_offset = SEED as f64 * 1000.0;
    let biome = biome_system.get_biome_at(x, z);

    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = NOISE_FREQ as f64;
    let mut max_value = 0.0;

    for _ in 0..biome.octaves {
        let nx = (x as f64) * frequency + seed_offset;
        let nz = (z as f64) * frequency + seed_offset;
        let noise_val = noise.get([nx, nz]);

        total += noise_val * amplitude;
        max_value += amplitude;

        amplitude *= biome.persistence as f64;
        frequency *= LACUNARITY;
    }

    let normalized = total / max_value;
    let height =
        biome.base_height as f64 + ((normalized + 1.0) / 2.0) * (biome.height_scale as f64);

    height as f32
}

fn build_mesh_data(
    coord: (i32, i32),
    heightmap: &Vec<Vec<f32>>,
    biome_system: &BiomeSystem,
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

    // Generate vertex colors based on biome
    let chunk_world_x = coord.0 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
    let chunk_world_z = coord.1 as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

    for z in 0..grid_size {
        for x in 0..grid_size {
            let height = heightmap[z][x];
            let world_x = chunk_world_x + (x as f32 * TERRAIN_RESOLUTION);
            let world_z = chunk_world_z + (z as f32 * TERRAIN_RESOLUTION);

            let biome = biome_system.get_biome_at(world_x, world_z);
            let height_normalized =
                ((height - biome.base_height) / biome.height_scale).clamp(0.0, 1.0);
            let height_curved = height_normalized.powf(biome.color_transition_power);

            let r = (biome.base_color.r as f32
                + (biome.peak_color.r as f32 - biome.base_color.r as f32) * height_curved)
                as u8;
            let g = (biome.base_color.g as f32
                + (biome.peak_color.g as f32 - biome.base_color.g as f32) * height_curved)
                as u8;
            let b = (biome.base_color.b as f32
                + (biome.peak_color.b as f32 - biome.base_color.b as f32) * height_curved)
                as u8;

            colors.push(Color::new(r, g, b, 255));
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

fn compute_normals(heightmap: &[Vec<f32>]) -> Vec<[f32; 3]> {
    let grid_size = heightmap.len();
    let mut normals = Vec::with_capacity(grid_size * grid_size);
    let last = grid_size - 1;

    for z in 0..grid_size {
        for x in 0..grid_size {
            // Pick two adjacent X indices: forward if possible, backwards at right edge of chunk
            let (x_a, x_b) = if x < last {
                (x, x + 1) // Forward diff
            } else {
                (x - 1, x) // Backwards diff at last column
            };

            // Same thing for Z
            let (z_a, z_b) = if z < last { (z, z + 1) } else { (z - 1, z) };

            let nx = heightmap[z][x_a] - heightmap[z][x_b];
            let ny = TERRAIN_RESOLUTION;
            let nz = heightmap[z_a][x] - heightmap[z_b][x];

            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            normals.push([nx / len, ny / len, nz / len]);
        }
    }

    normals
}
