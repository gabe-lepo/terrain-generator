use shared::Player;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct Client {
    pub id: Uuid,
    pub session: u8,
    pub addr: SocketAddr,
    pub tx: mpsc::Sender<String>,
    pub player: Player,
}

impl Client {
    pub fn new(addr: SocketAddr, session: u8, tx: mpsc::Sender<String>) -> Self {
        Self {
            id: Self::derive_id(addr),
            session,
            addr,
            tx,
            player: Player::new(),
        }
    }

    fn derive_id(addr: SocketAddr) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, addr.to_string().as_bytes())
    }
}
