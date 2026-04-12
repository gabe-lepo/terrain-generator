use crate::chunk::{CHUNK_SIZE, Chunk, ChunkCoord};
use crate::world::WorldQuery;

use noise::Perlin;
use raylib::prelude::*;
use std::collections::HashMap;

const VIEW_DISTANCE: i32 = 20;
const RENDER_WIREFRAME: bool = true;

pub struct TerrainManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    noise: Perlin,
    seed_offset: f64,
    last_player_chunk: Option<ChunkCoord>,
}

impl TerrainManager {
    pub fn new(seed: u32) -> Self {
        let noise = Perlin::new(seed);
        let seed_offset = seed as f64 * 1000.0;

        Self {
            chunks: HashMap::new(),
            noise,
            seed_offset,
            last_player_chunk: None,
        }
    }

    /// Update which chunks are loaded based on player pos
    pub fn update(&mut self, player_pos: Vector3, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let current_chunk = ChunkCoord::from_world_pos(player_pos.x, player_pos.z);

        // Only update chunks if player moved to different chunk
        if self.last_player_chunk == Some(current_chunk) {
            return;
        }

        self.last_player_chunk = Some(current_chunk);

        // Determine which chunks should load
        let mut chunks_to_keep = std::collections::HashSet::new();

        for dx in -VIEW_DISTANCE..=VIEW_DISTANCE {
            for dz in -VIEW_DISTANCE..=VIEW_DISTANCE {
                let chunk_coord = ChunkCoord::new(current_chunk.x + dx, current_chunk.z + dz);
                chunks_to_keep.insert(chunk_coord);

                // Load chunk if not already
                if !self.chunks.contains_key(&chunk_coord) {
                    println!("Loading chunk {:?}", chunk_coord);
                    let chunk =
                        Chunk::generate(chunk_coord, &self.noise, self.seed_offset, rl, thread);
                    self.chunks.insert(chunk_coord, chunk);
                }
            }
        }

        // Unload chunks outside view distance
        self.chunks.retain(|coord, _| {
            let should_keep = chunks_to_keep.contains(coord);
            if !should_keep {
                println!("Unloading chunk {:?}", coord);
            }
            should_keep
        });
    }

    pub fn render(&self, d: &mut RaylibMode3D<RaylibDrawHandle>) {
        for chunk in self.chunks.values() {
            chunk.render(d, RENDER_WIREFRAME);
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

impl WorldQuery for TerrainManager {
    fn get_height_at(&self, x: f32, z: f32) -> f32 {
        // Determine which chunk contains this position
        let chunk_coord = ChunkCoord::from_world_pos(x, z);

        // If chunk is loaded, used cached heightmap
        // Otherwise calc directly from noise
        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            // Convert world pos to local chunk coord
            let (chunk_world_x, chunk_world_z) = chunk_coord.to_world_pos();
            let local_x = x - chunk_world_x;
            let local_z = z - chunk_world_z;

            chunk.get_height_at_local(local_x, local_z)
        } else {
            // Chunk not loaded, calc from noise
            self.calculate_height_from_noise(x, z)
        }
    }
}

impl TerrainManager {
    fn calculate_height_from_noise(&self, x: f32, z: f32) -> f32 {
        use crate::chunk::get_height;
        get_height(x, z, &self.noise, self.seed_offset)
    }
}
