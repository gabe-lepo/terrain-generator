use shared::{ClientMessage, ServerMessage, Vec3};
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};
use uuid::Uuid;

/// Configuration for server connection, will be user config later
pub struct ServerConfig {
    pub address: Ipv4Addr,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: Ipv4Addr::new(127, 0, 0, 1),
            port: 8080,
        }
    }
}

/// Messages sent from the network task to the main game loop
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Connected,
    Disconnected,
    PlayerPositionUpdate { player_id: Uuid, position: Vec3 },
}

/// Handle for sending commands to network task
pub struct NetworkHandle {
    tx: mpsc::UnboundedSender<ClientMessage>,
}

impl NetworkHandle {
    /// Send a position update to the server
    pub fn send_position_update(&self, position: Vec3) {
        let msg = ClientMessage::PositionUpdate { position };
        // Ignore send errors for now (network task may be dead)
        // TODO: Handle errors

        let _ = self.tx.send(msg);
    }
}

/// Spawn the network task and return a handle + event receiver
pub fn spawn_network_task(
    config: ServerConfig,
) -> (NetworkHandle, mpsc::UnboundedReceiver<NetworkEvent>) {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

    // Spawn the tokio runtime in a separate thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            network_task(config, event_tx, cmd_rx).await;
        });
    });

    let handle = NetworkHandle { tx: cmd_tx };
    (handle, event_rx)
}

/// Main network task
async fn network_task(
    config: ServerConfig,
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
    mut cmd_rx: mpsc::UnboundedReceiver<ClientMessage>,
) {
    loop {
        // Connect
        let addr = format!("{}:{}", config.address, config.port);
        println!("Connecting to server at {}", addr);

        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                println!("Connected to server!");
                let _ = event_tx.send(NetworkEvent::Connected);

                // Handle connection
                if let Err(e) = handle_connection(stream, &event_tx, &mut cmd_rx).await {
                    println!("Connection error: {e}");
                }

                let _ = event_tx.send(NetworkEvent::Disconnected);
            }
            Err(e) => {
                println!("Failed to connect: {e}");
            }
        }

        // Wait before reconnecting
        println!("Reconnecting in 3 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
}

/// Handle a single conn to server
async fn handle_connection(
    stream: TcpStream,
    event_tx: &mpsc::UnboundedSender<NetworkEvent>,
    cmd_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        tokio::select! {
            // REad incomign messages from server
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => {
                        // Server closed connection
                        println!("Server closed connection");
                        return Ok(());
                    }
                    Ok(_) => {
                        // Parse server message
                        let trimmed = line.trim();
                        match serde_json::from_str::<ServerMessage>(trimmed) {
                            Ok(ServerMessage::PositionUpdate {player_id, position}) => {
                                let _ = event_tx.send(NetworkEvent::PlayerPositionUpdate {player_id, position});
                            }
                            Err(e) => {
                                println!("Failed to parse server message: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        println!("read error: {e}");
                        return Err(e.into());
                    }
                }
            }

            // Send outgoing messages to server
            Some(msg) = cmd_rx.recv() => {
                let json = serde_json::to_string(&msg)?;
                writer.write_all(json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        }
    }
}
