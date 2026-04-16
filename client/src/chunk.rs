use crate::biome::BiomeSystem;
use crate::chunk_loader::ChunkData;
use crate::config::{CHUNK_SIZE, LACUNARITY, NOISE_FREQ, SEED, TERRAIN_RESOLUTION};

use std::f32;

use noise::{NoiseFn, Perlin};
use raylib::ffi;
use raylib::prelude::*;

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
    pub fn from_data(
        data: ChunkData,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        fog_shader: Option<&Shader>,
    ) -> Self {
        let coord = ChunkCoord::new(data.coord.0, data.coord.1);
        let vertices = data.vertices;
        let indices = data.indices;
        let colors = data.colors;
        let bounding_box = data.bounding_box;
        let normals = data.normals;

        // WARN: Unsafe FFI calls
        // Build mesh from pre-generated data
        let mesh = unsafe {
            let vertex_count = vertices.len() as i32;
            let triangle_count = (indices.len() / 3) as i32;

            // Alloc mem for mesh data
            let vertices_flat: Vec<f32> =
                vertices.iter().flat_map(|v| vec![v.x, v.y, v.z]).collect();

            let colors_flat: Vec<u8> = colors
                .iter()
                .flat_map(|c| vec![c.r, c.g, c.b, c.a])
                .collect();

            // Normals (simple up pointing for now)
            let normals_flat: Vec<f32> = normals.iter().flat_map(|n| [n[0], n[1], n[2]]).collect();

            // Copy
            let vertices_ptr =
                libc::malloc(vertices_flat.len() * std::mem::size_of::<f32>()) as *mut f32;
            let indices_ptr = libc::malloc(indices.len() * std::mem::size_of::<u16>()) as *mut u16;
            let normals_ptr =
                libc::malloc(normals_flat.len() * std::mem::size_of::<f32>()) as *mut f32;
            let colors_ptr = libc::malloc(colors_flat.len() * std::mem::size_of::<u8>()) as *mut u8;

            std::ptr::copy_nonoverlapping(
                vertices_flat.as_ptr(),
                vertices_ptr,
                vertices_flat.len(),
            );
            std::ptr::copy_nonoverlapping(indices.as_ptr(), indices_ptr, indices.len());
            std::ptr::copy_nonoverlapping(normals_flat.as_ptr(), normals_ptr, normals_flat.len());
            std::ptr::copy_nonoverlapping(colors_flat.as_ptr(), colors_ptr, colors_flat.len());

            let mut mesh = ffi::Mesh {
                vertexCount: vertex_count,
                triangleCount: triangle_count,
                vertices: vertices_ptr,
                indices: indices_ptr,
                normals: normals_ptr,
                colors: colors_ptr,
                texcoords: std::ptr::null_mut(),
                texcoords2: std::ptr::null_mut(),
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

            // Upload to GPU
            ffi::UploadMesh(&mut mesh, false);
            Mesh::from_raw(mesh)
        };

        let mut model = rl
            .load_model_from_mesh(thread, unsafe { mesh.make_weak() })
            .expect("Failed to create model from mesh");

        // Set fog shader on the model's material if provided
        if let Some(shader) = fog_shader {
            let materials = model.materials_mut();
            if let Some(material) = materials.get_mut(0) {
                material.as_mut().shader = shader.as_ref().clone();
            }
        }

        Self {
            coord,
            model,
            heightmap: data.heightmap,
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

        if render_wireframe {
            // Wireframe only - no fog possible on lines
            d.draw_model_wires(&self.model, position, 1.0, Color::BLACK);
        } else {
            // Solid model with fog shader applied via material
            d.draw_model(&self.model, position, 1.0, Color::WHITE);
        }
    }

    pub fn get_height_at(&self, local_x: f32, local_z: f32) -> f32 {
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
}

/// Fancy terrain gen
pub fn get_height(x: f32, z: f32, noise: &Perlin, biome_system: &BiomeSystem) -> f32 {
    let seed_offset = SEED as f64 * 1000.0;

    // Sample biome as this position
    let biome = biome_system.get_biome_at(x, z);

    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = NOISE_FREQ;
    let mut max_value = 0.0; // Normalization

    for _ in 0..biome.octaves {
        let nx = (x as f64) * frequency + seed_offset;
        let nz = (z as f64) * frequency + seed_offset;
        let noise_val = noise.get([nx, nz]);

        total += noise_val * amplitude;
        max_value += amplitude;

        // Biome params
        amplitude *= biome.persistence as f64;
        frequency *= LACUNARITY;
    }

    let normalized = total / max_value;
    let height =
        biome.base_height as f64 + ((normalized + 1.0) / 2.0) * (biome.height_scale as f64);

    height as f32
}
