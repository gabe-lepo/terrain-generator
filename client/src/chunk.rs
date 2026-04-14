use crate::biome::BiomeSystem;
use crate::chunk_loader::ChunkData;
use crate::config::{CHUNK_SIZE, TERRAIN_RESOLUTION, NOISE_FREQ, LACUNARITY, SEED};

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
    /// Generate chunk at given chunk coord
    pub fn generate(
        coord: ChunkCoord,
        noise: &Perlin,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        biome_system: &BiomeSystem,
    ) -> Self {
        let heightmap = Self::generate_heightmap(coord, noise, biome_system);
        let mesh = Self::build_mesh(coord, &heightmap, biome_system);
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
            let normals_flat: Vec<f32> = (0..vertices.len())
                .flat_map(|_| vec![0.0, 1.0, 0.0])
                .collect();

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

        // We dont need to recalc heightmap since we wont use it for height queries
        // Height queries will fall back to noise for async loaded chunks
        Self {
            coord,
            model,
            heightmap: vec![], // We dont need it
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

    /// Generate heightmap for the chunk
    fn generate_heightmap(
        coord: ChunkCoord,
        noise: &Perlin,
        biome_system: &BiomeSystem,
    ) -> Vec<Vec<f32>> {
        let grid_size = CHUNK_SIZE as usize + 1;
        let mut heightmap = Vec::with_capacity(grid_size);

        let (chunk_world_x, chunk_world_z) = coord.to_world_pos();

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
    fn build_mesh(
        coord: ChunkCoord,
        heightmap: &Vec<Vec<f32>>,
        biome_system: &BiomeSystem,
    ) -> Mesh {
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

        // Gen vertex colors based on height and biome
        // TODO: This is getting complex, offload to another builder func
        for z in 0..grid_size {
            for x in 0..grid_size {
                let height = heightmap[z][x];

                // Get world position for the vertex
                let (chunk_world_x, chunk_world_z) = coord.to_world_pos();
                let world_x = chunk_world_x + (x as f32 * TERRAIN_RESOLUTION);
                let world_z = chunk_world_z + (z as f32 * TERRAIN_RESOLUTION);

                // Sample biome at this pos
                let biome = biome_system.get_biome_at(world_x, world_z);

                // Height-based blend (0-1 normalized)
                let height_range = biome.height_scale;
                let height_normalized =
                    ((height - biome.base_height) / height_range).clamp(0.0, 1.0);

                // Apply power curve to transition
                let height_curved = height_normalized.powf(biome.color_transition_power);

                // Blend between base and peak color based on height
                let r = (biome.base_color.r as f32
                    + (biome.peak_color.r as f32 - biome.base_color.r as f32) * height_curved)
                    as u8;
                let g = (biome.base_color.g as f32
                    + (biome.peak_color.g as f32 - biome.base_color.g as f32) * height_curved)
                    as u8;
                let b = (biome.base_color.b as f32
                    + (biome.peak_color.b as f32 - biome.base_color.b as f32) * height_curved)
                    as u8;

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
pub fn get_height(
    x: f32,
    z: f32,
    noise: &Perlin,
    biome_system: &BiomeSystem,
) -> f32 {
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
