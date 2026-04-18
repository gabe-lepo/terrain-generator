use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    PositionUpdate { position: NetworkVec3 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    PositionUpdate {
        player_id: Uuid,
        position: NetworkVec3,
    },
    PlayerDisconnected {
        player_id: Uuid,
    },
    WorldSync {
        seed: u64,
        hour: f32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl NetworkVec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub position: NetworkVec3,
}

impl Player {
    pub fn new() -> Self {
        Self {
            position: NetworkVec3::zero(),
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
