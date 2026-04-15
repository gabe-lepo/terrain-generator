# Terrain Generator

A multiplayer 3D terrain explorer with procedural terrain generation built
in Rust.

## TODO

### Server

- Create IP whitelisting
- Review the whole IP:PORT based UUID situation, client ports will change
  frequently need another deterministic way to give unique but persistent
  client ids
- Expose server to internet

### Client

- Menu system
- Menu-based configs
  - Allow user to enter server IP and port before connecting
- Auto update system
  - client contacts server with version
  - server says update avail or not,
  - client downloads + installs
- Shader-based lighting
- Objects! I.e. something to do in the game...
- Fix obvious visual lines between biomes (color and height)
- Move configs to shared, in case we need more complex server configs

## Running

```bash
# Server
cargo run -p server

# Client
cargo run -p client
```

The server listens on `127.0.0.1:8080`. Toggle `CONNECT` in
`client/src/config.rs` to run the client offline.

## Architecture

Cargo workspace with three members:

- `shared/` — message types (`ClientMessage`, `ServerMessage`) with
  tagged JSON serialization
- `server/` — async Tokio TCP server, broadcasts position updates to all
  other clients
- `client/` — raylib first-person renderer with async procedural terrain,
  biome system, and networking

Terrain is generated in chunks (16x16 vertices) across a 4-thread pool.
Three biomes (mountains, plains, hills) blend based on low-frequency
noise. A GLSL fog shader is applied to all chunks at some max render distance.

All tunable constants live in `client/src/config.rs`.
