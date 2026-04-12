use std::f32;

use noise::{NoiseFn, Perlin};
use raylib::ffi;
use raylib::prelude::*;

// Consts
pub const CHUNK_SIZE: i32 = 32;
pub const TERRAIN_RESOLUTION: f32 = 2.5;
const HEIGHT_SCALE: f32 = 150.0;
const NOISE_FREQ: f32 = 0.01;
const OCTAVES: i32 = 5; // Num layers
const LACUNARITY: f64 = 2.0; // Frequency multiplier
const PERSISTENCE: f64 = 0.5; // Amplitude multiplier

/// Chunk coordinates (not world coords!)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Convert world position to chunk coordinate
    pub fn from_world_pos(world_x: f32, world_z: f32) -> Self {
        let chunk_x = (world_x / (CHUNK_SIZE as f32 * TERRAIN_RESOLUTION)).floor() as i32;
        let chunk_z = (world_z / (CHUNK_SIZE as f32 * TERRAIN_RESOLUTION)).floor() as i32;

        Self::new(chunk_x, chunk_z)
    }

    /// Get world position of chunk origin (bottom left corner)
    pub fn to_world_pos(&self) -> (f32, f32) {
        let world_x = self.x as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
        let world_z = self.z as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

        (world_x, world_z)
    }
}

pub struct Chunk {
    pub coord: ChunkCoord,
    pub model: Model, // Make public so we can access materials
    heightmap: Vec<Vec<f32>>,
    pub bounding_box: BoundingBox,
}

impl Chunk {
    /// Generate chunk at given chunk coord
    pub fn generate(
        coord: ChunkCoord,
        noise: &Perlin,
        seed_offset: f64,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Self {
        let heightmap = Self::generate_heightmap(coord, noise, seed_offset);
        let mesh = Self::build_mesh(&heightmap);
        let model = rl
            .load_model_from_mesh(thread, unsafe { mesh.make_weak() })
            .expect("Failed to create model from mesh");
        let bounding_box = Self::calculate_bounding_box(&heightmap, coord);

        Self {
            coord,
            model,
            heightmap,
            bounding_box,
        }
    }

    pub fn render(
        &self,
        d: &mut RaylibMode3D<RaylibDrawHandle>,
        render_wireframe: bool,
        _shader: Option<&mut Shader>,
    ) {
        let (world_x, world_z) = self.coord.to_world_pos();
        let position = Vector3::new(world_x, 0.0, world_z);

        d.draw_model(&self.model, position, 1.0, Color::WHITE);

        if render_wireframe {
            d.draw_model_wires(&self.model, position, 1.0, Color::BLACK);
        }
    }

    /// Generate heightmap for the chunk
    fn generate_heightmap(coord: ChunkCoord, noise: &Perlin, seed_offset: f64) -> Vec<Vec<f32>> {
        let grid_size = CHUNK_SIZE as usize + 1;
        let mut heightmap = Vec::with_capacity(grid_size);

        let (chunk_world_x, chunk_world_z) = coord.to_world_pos();

        for z in 0..grid_size {
            let mut row = Vec::with_capacity(grid_size);
            for x in 0..grid_size {
                let world_x = chunk_world_x + (x as f32 * TERRAIN_RESOLUTION);
                let world_z = chunk_world_z + (z as f32 * TERRAIN_RESOLUTION);
                let height = get_height(world_x, world_z, noise, seed_offset);
                row.push(height);
            }
            heightmap.push(row);
        }

        heightmap
    }

    pub fn get_height_at_local(&self, local_x: f32, local_z: f32) -> f32 {
        // Convert local coords to grid coords
        let grid_x = local_x / TERRAIN_RESOLUTION;
        let grid_z = local_z / TERRAIN_RESOLUTION;

        // Get grid square corners
        let x0 = grid_x.floor() as i32;
        let z0 = grid_z.floor() as i32;
        let x1 = x0 + 1;
        let z1 = z0 + 1;

        let grid_size = self.heightmap.len() as i32;

        // Bounds check
        if x0 < 0 || x1 >= grid_size || z0 < 0 || z1 >= grid_size {
            return 0.0;
        }

        // Get 4 corner heights
        let h00 = self.heightmap[z0 as usize][x0 as usize];
        let h10 = self.heightmap[z0 as usize][x1 as usize];
        let h01 = self.heightmap[z1 as usize][x0 as usize];
        let h11 = self.heightmap[z1 as usize][x1 as usize];

        // Calc interpolation weights
        let fx = grid_x - (x0 as f32);
        let fz = grid_z - (z0 as f32);

        // Bilinear interp
        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;
        let height = h0 * (1.0 - fz) + h1 * fz;

        height
    }

    // Private
    fn build_mesh(heightmap: &Vec<Vec<f32>>) -> Mesh {
        let grid_size = heightmap.len();
        let vertex_count = grid_size * grid_size;
        let triangle_count = (grid_size - 1) * (grid_size - 1) * 2;

        let mut vertices = Vec::with_capacity(vertex_count * 3);
        let mut indices = Vec::with_capacity(triangle_count * 3);
        let mut normals = Vec::with_capacity(vertex_count * 3);
        let mut colors = Vec::with_capacity(vertex_count * 4);

        // Generate vertices (relative to chunk origin at 0,0,0)
        for z in 0..grid_size {
            for x in 0..grid_size {
                let local_x = x as f32 * TERRAIN_RESOLUTION;
                let local_z = z as f32 * TERRAIN_RESOLUTION;
                let height = heightmap[z][x];

                vertices.push(local_x);
                vertices.push(height);
                vertices.push(local_z);
            }
        }

        // Generate triangle indices
        for z in 0..(grid_size - 1) {
            for x in 0..(grid_size - 1) {
                let top_left = (z * grid_size + x) as u16;
                let top_right = top_left + 1;
                let bottom_left = ((z + 1) * grid_size + x) as u16;
                let bottom_right = bottom_left + 1;

                // First triangle (topleft, bottom left, topright)
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // Second triangle (topright, bottom left, bottom right)
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        // Calc normals (simple up pointing for now)
        for _ in 0..vertex_count {
            normals.push(0.0);
            normals.push(1.0);
            normals.push(0.0);
        }

        // Gen vertex colors based on height
        for z in 0..grid_size {
            for x in 0..grid_size {
                let height = heightmap[z][x];
                let normalized_height = height / HEIGHT_SCALE;
                let color_height = normalized_height.powf(3.5);

                // Color to white (low to high) gradient
                let r = (0.0 + color_height * 255.0) as u8;
                let g = (100.0 + color_height * 155.0) as u8;
                let b = (0.0 + color_height * 255.0) as u8;

                colors.push(r);
                colors.push(g);
                colors.push(b);
                colors.push(255);
            }
        }

        // Now build raylib mesh
        // WARN: Unsafe block (unsafe FFI)
        unsafe {
            // Alloc mem using libc malloc
            let vertices_ptr =
                libc::malloc(vertices.len() * std::mem::size_of::<f32>()) as *mut f32;
            let indices_ptr = libc::malloc(indices.len() * std::mem::size_of::<u16>()) as *mut u16;
            let normals_ptr = libc::malloc(normals.len() * std::mem::size_of::<f32>()) as *mut f32;
            let colors_ptr = libc::malloc(colors.len() * std::mem::size_of::<u8>()) as *mut u8;

            // Copy
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), vertices_ptr, vertices.len());
            std::ptr::copy_nonoverlapping(indices.as_ptr(), indices_ptr, indices.len());
            std::ptr::copy_nonoverlapping(normals.as_ptr(), normals_ptr, normals.len());
            std::ptr::copy_nonoverlapping(colors.as_ptr(), colors_ptr, colors.len());

            // Construct the mesh
            let mut mesh = ffi::Mesh {
                vertexCount: vertex_count as i32,
                triangleCount: triangle_count as i32,
                vertices: vertices_ptr,
                indices: indices_ptr,
                normals: normals_ptr,
                // All others are null or 0
                texcoords: std::ptr::null_mut(),
                texcoords2: std::ptr::null_mut(),
                colors: colors_ptr,
                tangents: std::ptr::null_mut(),
                animVertices: std::ptr::null_mut(),
                animNormals: std::ptr::null_mut(),
                boneIds: std::ptr::null_mut(),
                boneWeights: std::ptr::null_mut(),
                boneMatrices: std::ptr::null_mut(),
                boneCount: 0,
                vaoId: 0,
                vboId: std::ptr::null_mut(),
            };

            // Fire off to GPU
            ffi::UploadMesh(&mut mesh, false);

            // Wrap in Mesh
            Mesh::from_raw(mesh)
        } // WARN: Unsafe end
    }

    fn calculate_bounding_box(heightmap: &Vec<Vec<f32>>, coord: ChunkCoord) -> BoundingBox {
        let (world_x, world_z) = coord.to_world_pos();

        // Find min/max heights in heightmap
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
}

/// Fancy terrain gen
pub fn get_height(x: f32, z: f32, noise: &Perlin, seed_offset: f64) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = NOISE_FREQ as f64;
    let mut max_value = 0.0; // Normalization

    for _ in 0..OCTAVES {
        let nx = (x as f64) * frequency + seed_offset;
        let nz = (z as f64) * frequency + seed_offset;
        let noise_val = noise.get([nx, nz]);

        total += noise_val * amplitude;
        max_value += amplitude;

        amplitude *= PERSISTENCE;
        frequency *= LACUNARITY;
    }

    let normalized = total / max_value;
    let height = ((normalized + 1.0) / 2.0) * (HEIGHT_SCALE as f64);

    height as f32
}
