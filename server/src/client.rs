use shared::Player;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct Client {
    pub id: Uuid,
    pub session: u32,
    pub _addr: SocketAddr,
    pub tx: mpsc::Sender<String>,
    pub _player: Player,
}

impl Client {
    pub fn new(addr: SocketAddr, session: u32, tx: mpsc::Sender<String>) -> Self {
        Self {
            id: Self::derive_id(addr),
            session,
            _addr: addr,
            tx,
            _player: Player::new(),
        }
    }

    fn derive_id(addr: SocketAddr) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, addr.to_string().as_bytes())
    }
}
