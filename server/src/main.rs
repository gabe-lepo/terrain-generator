mod client;

use client::Client;
use std::sync::{Arc, Mutex};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::mpsc,
};

use uuid::Uuid;

use shared::{ClientMessage, ServerMessage};

// TODO: Confirm if is enough for 2 players and 20hz? update freq
const MAX_MESSAGES: usize = 32;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("listener panicked!");
    println!("Server listening on port 8080\n");

    // Shared state machine setup
    let registry: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    let mut next_session: u32 = 1;

    // Main accept loop
    loop {
        // Accept connection
        let (socket, addr) = listener.accept().await.expect("couldnt accept listener");

        // Register client in shared state
        let (tx, rx) = mpsc::channel::<String>(MAX_MESSAGES);

        let mut registry_guard = registry
            .lock()
            .expect("registry_guard lock failure (outer)");
        let session = next_session;
        next_session = next_session.wrapping_add(1);
        let client = Client::new(addr, session, tx);
        let client_id = client.id;
        registry_guard.push(client);
        drop(registry_guard);

        let registry_clone = Arc::clone(&registry);

        println!(
            "New client connection:\n\tFrom: {addr}\n\tClient ID: {client_id}\n\tSession: {session}\n"
        );
        // Thread the connection handler
        tokio::spawn(async move {
            handle_connection(socket, client_id, rx, registry_clone).await;
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    client_id: Uuid,
    mut rx: mpsc::Receiver<String>,
    registry: Arc<Mutex<Vec<Client>>>,
) {
    let session = {
        let registry_guard = registry
            .lock()
            .expect("registry_guard lock failure (get session)");
        registry_guard
            .iter()
            .find(|c| c.id == client_id)
            .map(|c| c.session)
            .expect("couldnt find session for client")
    };
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        tokio::select! {
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => {
                        println!("Client {client_id} disconnected\n");
                        let msg = ServerMessage::PlayerDisconnected {player_id: client_id};
                        let serialized = serde_json::to_string(&msg).expect("player disconnect message serialization failure") + "\n";

                        let mut registry_guard = registry.lock().expect("registry_guard lock failure (inner - Ok0)");
                        for other in registry_guard.iter().filter(|c| c.id != client_id) {
                            // TODO: Handle Result<> from try_send properly...
                            let _  = other.tx.try_send(serialized.clone());
                        }
                        registry_guard.retain(|c| c.id != client_id);
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            match serde_json::from_str::<ClientMessage>(trimmed) {
                                Ok(ClientMessage::PositionUpdate {position}) => {
                                    let msg = ServerMessage::PositionUpdate {
                                        player_id: client_id,
                                        position,
                                    };
                                    let serialized = serde_json::to_string(&msg).expect("position update message serialization failure") + "\n";
                                    let registry_guard = registry.lock().expect("registry_guard lock failure (inner - Ok_)");
                                    for other in registry_guard.iter().filter(|c| c.id != client_id) {
                                        // TODO: Handle Result<> from try_send properly...
                                        let _ = other.tx.try_send(serialized.clone());
                                    }
                                    println!("PositionUpdate:\n\tClient: {client_id}\n\tSession: {session}\n\tNew Position: x:{}|y:{}|z:{}\n",
                                        position.x,
                                        position.y,
                                        position.z
                                    );
                                    // TODO: We dont actually update the player pos yet!
                                }
                                Err(e) => {
                                    println!("Invalid message!\n\tClient: {client_id}\n\tSession: {session}\n\tmsg: {trimmed}\n\tError: {e}\n");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading: {e}");
                        break;
                    }
                }
            }
            Some(msg) = rx.recv() => {
                writer.write_all(msg.as_bytes()).await.expect("Couldnt write to writer");
            }
        }
    }
}
