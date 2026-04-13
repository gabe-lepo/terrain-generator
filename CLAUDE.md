# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## Project Structure

This is a Cargo workspace with three members:

- `shared/` — shared types between server and client: `ClientMessage`,
  `ServerMessage`, `Vec3`, `Player`. All message enums use
  `#[serde(tag = "type")]` for tagged JSON serialization.
- `server/` — async multiplayer game server built on Tokio
- `client/` — multiplayer 3D terrain explorer with raylib rendering,
  procedural terrain generation, and networked multiplayer

## Commands

Run from workspace root:

```bash
cargo build              # build all workspace members
cargo run -p server      # run server
cargo run -p client      # run client (must be run from client/ dir for
                         # shader paths to resolve)
cargo test               # run all tests
cargo test <name>        # run single test by name
cargo clippy             # lint
cargo fmt                # format
```

The server listens on `127.0.0.1:8080`. The client expects shader files
at `client/shaders/` relative to the working directory.

## Architecture

### Server

The server uses an async tokio runtime with the following architecture:

**Connection handling:**
- Main loop accepts TCP connections and spawns a task per client
- Each client gets assigned a UUID (derived via UUIDv5 from socket
  address + NAMESPACE_DNS) and a monotonic session number
- Client registry is `Arc<Mutex<Vec<Client>>>` shared across all tasks

**Message flow:**
- Protocol is newline-delimited JSON
- Each client has an `mpsc::channel` (capacity: `MAX_MESSAGES = 32`) for
  outbound messages
- Per-client task uses `tokio::select!` to multiplex:
  - Inbound: `AsyncBufReadExt::read_line` from socket
  - Outbound: `mpsc::Receiver::recv` for messages to send
- When a client sends `ClientMessage::PositionUpdate`, server
  rebroadcasts as `ServerMessage::PositionUpdate` (with `player_id`) to
  all other clients via `try_send` (non-blocking)

**Client identity:**
- UUID derivation means two clients from the same socket address get the
  same UUID (noted in code as potential issue)
- Each connection gets a unique session number regardless of UUID
- On disconnect, client is removed from registry via `retain`

### Client

The client is a multiplayer 3D first-person terrain explorer built on
raylib with the following architecture:

**Terrain system:**
- Infinite procedural terrain via chunk-based streaming
  (`TerrainManager`)
- Each `Chunk` is a 32x32 grid at 2.5 unit resolution, generated from
  multi-octave Perlin noise heightmap (5 octaves, lacunarity 2.0,
  persistence 0.5)
- Biome system (`biome.rs`) divides world into three biomes (mountains,
  plains, hills) based on low-frequency noise, with smooth parameter
  blending between biomes
- Chunks load/unload based on player position (VIEW_DISTANCE = 25 chunks)
- Frustum culling using per-chunk bounding boxes skips rendering chunks
  outside camera view
- `WorldQuery` trait allows player physics to query terrain height via
  bilinear interpolation of heightmap

**Biome system:**
- Three biome types with distinct characteristics:
  - Mountains: high amplitude (180), 6 octaves, gray→white gradient
  - Plains: low amplitude (40), 3 octaves, tan/wheat colors
  - Hills: medium amplitude (100), 5 octaves, green gradient
- Each biome defines: height_scale, base_height, octaves, persistence,
  base_color, peak_color, color_transition_power
- `color_transition_power` controls gradient curve (plains ≈0.3 for
  mostly flat color, mountains ≈3.5 for sharp snowcaps)
- Biomes blend smoothly via linear interpolation in transition zones

**Mesh generation:**
- Heightmap → vertices/indices → raylib `Mesh` via unsafe FFI
- Memory allocated with `libc::malloc`, uploaded to GPU with
  `UploadMesh`
- Vertex colors based on biome palette + height within biome range (uses
  power curve for transition)
- Colors sampled per-vertex, causing smooth blending between biomes

**Rendering:**
- Distance fog shader (`client/shaders/fog.{vs,fs}`) applied to all chunk
  models via material shader assignment
- Shader uniforms (camera position, fog distances, fog color) updated per
  frame via `ShaderManager` and unsafe `SetShaderValue` FFI calls
- Fog distances calculated as percentage of max view distance
  (fog_near = 40%, fog_far = 50%)
- RENDER_WIREFRAME const switches between solid and wireframe rendering

**Player controller:**
- First-person camera with mouse look (yaw/pitch) and WASD movement
- Physics: gravity, jumping, ground detection with smooth terrain
  snapping after horizontal movement
- GOD_MODE constant bypasses physics for free flight (space/ctrl for
  vertical)
- Sprint (Left Shift) and crouch (Left Control) modifiers

**Networking:**
- Separate thread runs tokio runtime for async TCP networking
  (`network.rs`)
- `NetworkHandle` sends position updates to server (every frame, ~60Hz)
- `NetworkEvent` channel receives server events (Connected, Disconnected,
  PlayerPositionUpdate)
- Auto-reconnect with 3-second delay on disconnect
- Remote players stored in `HashMap<Uuid, RemotePlayer>` and rendered as
  red cubes
- Non-blocking: `try_recv()` ensures network never blocks rendering
  thread
- `ServerConfig` struct holds connection details (default:
  127.0.0.1:8080), prepared for future menu-based configuration

### Message Types

All message enums in `shared/` use `#[serde(tag = "type")]` which
serializes as:

```json
{"type": "PositionUpdate", "position": {"x": 1.0, "y": 2.0, "z": 3.0}}
```

The `type` field acts as the enum discriminant in JSON.

## Client Module Structure

- `main.rs` — main loop, network event processing, rendering
  orchestration
- `player.rs` — first-person controller with physics
- `terrain_manager.rs` — chunk loading/unloading, rendering, height
  queries
- `chunk.rs` — mesh generation, heightmap storage, biome-based coloring
- `biome.rs` — biome definitions, parameter blending, color lerping
- `world.rs` — `WorldQuery` trait for terrain height sampling
- `shaders.rs` — fog shader loading and uniform management
- `network.rs` — async networking on separate thread, auto-reconnect
- `remote_player.rs` — remote player state and rendering (red cubes)

## Known Issues & TODOs

**Server:**
- Does not update `Client.player.position` in registry when receiving
  position updates (server/src/main.rs:111)
- If messages disappear, check `try_send` Result handling
  (server/src/main.rs:104)

**Client:**
- Cursor disabled in main.rs (line 41 commented out for development)
- Dead code warnings allowed (line 2) - should remove after development
- Remote players rendered as simple cubes (TODO in remote_player.rs:20)
- No menu system for server IP/port configuration yet (hardcoded default
  in ServerConfig)
- Network error handling minimal (ignores send failures in
  NetworkHandle::send_position_update)
