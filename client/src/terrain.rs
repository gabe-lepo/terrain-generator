use crate::world::WorldQuery;

use noise::{NoiseFn, Perlin};
use raylib::prelude::*;

// Terrain gen params
const TERRAIN_SIZE: i32 = 200;
const TERRAIN_RESOLUTION: f32 = 1.0;
const HEIGHT_SCALE: f32 = 80.0;
const NOISE_FREQ: f32 = 0.015;
const RENDER_WIREFRAME: bool = true;

pub struct Terrain {
    model: Model,
    heightmap: Vec<Vec<f32>>,
    seed: u32,
}

impl Terrain {
    // Public
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread, seed: u32) -> Self {
        let noise = Perlin::new(seed);
        let seed_offset = seed as f64 + 1000.0;

        // gen heightmap
        let heightmap = Self::generate_heightmap(&noise, seed_offset);

        // Build mesh and model
        let mesh = Self::build_mesh(&heightmap, rl, thread);
        let model = rl
            .load_model_from_mesh(thread, unsafe { mesh.make_weak() })
            .expect("Failed to create model from mesh");

        Self {
            model,
            heightmap,
            seed,
        }
    }

    pub fn render(&self, d: &mut RaylibMode3D<RaylibDrawHandle>) {
        // Terrain model
        d.draw_model(&self.model, Vector3::zero(), 1.0, Color::WHITE);

        // Wireframe
        if RENDER_WIREFRAME {
            d.draw_model_wires(&self.model, Vector3::zero(), 1.0, Color::BLACK);
        }
    }

    // Private
    fn generate_heightmap(noise: &Perlin, seed_offset: f64) -> Vec<Vec<f32>> {
        let grid_size = (TERRAIN_SIZE as f32 / TERRAIN_RESOLUTION) as usize;
        let mut heightmap = Vec::with_capacity(grid_size);

        let offset = TERRAIN_SIZE as f32 / 2.0; // Centering offset

        for z in 0..grid_size {
            let mut row = Vec::with_capacity(grid_size);
            for x in 0..grid_size {
                let world_x = (x as f32 * TERRAIN_RESOLUTION) - offset;
                let world_z = (z as f32 * TERRAIN_RESOLUTION) - offset;
                let height = get_height(world_x, world_z, noise, seed_offset);
                row.push(height);
            }
            heightmap.push(row);
        }

        heightmap
    }

    fn build_mesh(heightmap: &Vec<Vec<f32>>, rl: &mut RaylibHandle, thread: &RaylibThread) -> Mesh {
        let grid_size = heightmap.len();
        let vertex_count = grid_size * grid_size;
        let triangle_count = (grid_size - 1) * (grid_size - 1) * 2;

        let mut vertices = Vec::with_capacity(vertex_count * 3);
        let mut indices = Vec::with_capacity(triangle_count * 3);
        let mut normals = Vec::with_capacity(vertex_count * 3);

        let offset = TERRAIN_SIZE as f32 / 2.0;

        // Generate vertices
        for z in 0..grid_size {
            for x in 0..grid_size {
                let world_x = (x as f32 * TERRAIN_RESOLUTION) - offset;
                let world_z = (z as f32 * TERRAIN_RESOLUTION) - offset;
                let height = heightmap[z][x];

                vertices.push(world_x);
                vertices.push(height);
                vertices.push(world_z);
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

        // Calc normals (flat shading for now)
        // TODO: For now, lets use simple up pointing normals
        for _ in 0..vertex_count {
            normals.push(0.0);
            normals.push(1.0); // Up vector
            normals.push(0.0);
        }

        // Vertex height-based colors for simple visual clarity on terrain
        let mut colors = Vec::with_capacity(vertex_count * 4); // RGBA
        for z in 0..grid_size {
            for x in 0..grid_size {
                let height = heightmap[z][x];
                let normalized_height = height / HEIGHT_SCALE;

                // logarithmic-like gradient curve
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
        // WARN: Unsafe block due to raylib C API & raylib-rs FFI impl
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

            // Fire to GPU
            ffi::UploadMesh(&mut mesh, false);

            // Wrap in Mesh
            Mesh::from_raw(mesh)
        } // WARN: Unsafe end
    }
}

impl WorldQuery for Terrain {
    fn get_height_at(&self, x: f32, z: f32) -> f32 {
        // world coords to grid indices
        let offset = TERRAIN_SIZE as f32 / 2.0;
        let grid_x = ((x + offset) / TERRAIN_RESOLUTION) as i32;
        let grid_z = ((z + offset) / TERRAIN_RESOLUTION) as i32;

        // Check bounds
        let grid_size = self.heightmap.len() as i32;
        if grid_x < 0 || grid_x >= grid_size || grid_z < 0 || grid_z >= grid_size {
            return 0.0;
        }

        // Return height at hte grid point
        self.heightmap[grid_z as usize][grid_x as usize]
    }
}

fn get_height(x: f32, z: f32, noise: &Perlin, seed_offset: f64) -> f32 {
    // Scale input coords by freq
    let nx = (x as f64) * (NOISE_FREQ as f64) + seed_offset;
    let nz = (z as f64) * (NOISE_FREQ as f64) + seed_offset;

    // Sample noise
    let noise_val = noise.get([nx, nz]);

    // Normalize from -1,1 -> 0,HEIGHT_SCALE
    let height = ((noise_val + 1.0) / 2.0) * (HEIGHT_SCALE as f64);

    height as f32
}
