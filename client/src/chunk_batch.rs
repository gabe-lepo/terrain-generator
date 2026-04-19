use crate::chunk_loader::BatchData;

use raylib::ffi;
use raylib::prelude::*;

pub const BATCH_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BatchCoord {
    pub gx: i32,
    pub gz: i32,
}

impl BatchCoord {
    pub fn from_chunk_coord(cx: i32, cz: i32) -> Self {
        Self {
            gx: cx.div_euclid(BATCH_SIZE as i32),
            gz: cz.div_euclid(BATCH_SIZE as i32),
        }
    }

    /// Chunk coord of the batch's top-left corner
    pub fn origin_chunk(&self) -> (i32, i32) {
        (self.gx * BATCH_SIZE as i32, self.gz * BATCH_SIZE as i32)
    }
}

pub struct ChunkBatch {
    pub coord: BatchCoord,
    model: Model,
    pub bbox: BoundingBox,
}

impl ChunkBatch {
    pub fn from_data(
        data: BatchData,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        terrain_shader: Option<&Shader>,
    ) -> Self {
        let coord = data.coord;

        let mesh = unsafe {
            let vertex_count = data.vertices.len() as i32;
            let triangle_count = vertex_count / 3;

            let vertices_flat: Vec<f32> =
                data.vertices.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
            let colors_flat: Vec<u8> = data
                .colors
                .iter()
                .flat_map(|c| [c.r, c.g, c.b, c.a])
                .collect();

            let vertices_ptr =
                libc::malloc(vertices_flat.len() * std::mem::size_of::<f32>()) as *mut f32;
            let colors_ptr = libc::malloc(colors_flat.len() * std::mem::size_of::<u8>()) as *mut u8;

            std::ptr::copy_nonoverlapping(
                vertices_flat.as_ptr(),
                vertices_ptr,
                vertices_flat.len(),
            );
            std::ptr::copy_nonoverlapping(colors_flat.as_ptr(), colors_ptr, colors_flat.len());

            let mut mesh = ffi::Mesh {
                vertexCount: vertex_count,
                triangleCount: triangle_count,
                vertices: vertices_ptr,
                indices: std::ptr::null_mut(),
                normals: std::ptr::null_mut(),
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

            ffi::UploadMesh(&mut mesh, false);
            Mesh::from_raw(mesh)
        };

        let mut model = rl
            .load_model_from_mesh(thread, unsafe { mesh.make_weak() })
            .expect("Failed to create model from mesh");

        if let Some(shader) = terrain_shader {
            let materials = model.materials_mut();
            if let Some(material) = materials.get_mut(0) {
                material.as_mut().shader = *shader.as_ref();
            }
        }

        Self {
            coord,
            model,
            bbox: data.bbox,
        }
    }

    pub fn render(&self, d: &mut RaylibMode3D<RaylibDrawHandle>, render_wireframe: bool) {
        if render_wireframe {
            d.draw_model_wires(&self.model, Vector3::zero(), 1.0, Color::BLACK);
        } else {
            d.draw_model(&self.model, Vector3::zero(), 1.0, Color::WHITE);
        }
    }
}
