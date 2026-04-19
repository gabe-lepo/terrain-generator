use crate::config::{CHUNK_SIZE, TERRAIN_RESOLUTION};

use std::f32;

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
    pub fn get_world_pos(&self) -> (f32, f32) {
        let world_x = self.x as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
        let world_z = self.z as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;

        (world_x, world_z)
    }
}

pub fn sample_heightmap(heightmap: &[Vec<f32>], local_x: f32, local_z: f32) -> f32 {
    let grid_x = local_x / TERRAIN_RESOLUTION;
    let grid_z = local_z / TERRAIN_RESOLUTION;

    let x0 = grid_x.floor() as i32;
    let z0 = grid_z.floor() as i32;
    let x1 = x0 + 1;
    let z1 = z0 + 1;

    let grid_size = heightmap.len() as i32;

    if x0 < 0 || x1 >= grid_size || z0 < 0 || z1 >= grid_size {
        return 0.0;
    }

    let h00 = heightmap[z0 as usize][x0 as usize];
    let h10 = heightmap[z0 as usize][x1 as usize];
    let h01 = heightmap[z1 as usize][x0 as usize];
    let h11 = heightmap[z1 as usize][x1 as usize];

    let fx = grid_x - x0 as f32;
    let fz = grid_z - z0 as f32;

    let h0 = h00 * (1.0 - fx) + h10 * fx;
    let h1 = h01 * (1.0 - fx) + h11 * fx;
    h0 * (1.0 - fz) + h1 * fz
}
