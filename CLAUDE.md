# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## Project Structure

This is a Cargo workspace with three members:

- `shared/` — shared types between server and client: `ClientMessage`,
  `ServerMessage`, `Vec3`, `Player`. All message enums use
  `#[serde(tag = "type")]` for tagged JSON serialization.
- `server/` — async multiplayer game server built on Tokio
- `client/` — 3D terrain explorer built with raylib, procedurally
  generated terrain using Perlin noise

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

The client is a 3D first-person terrain explorer built on raylib with the
following architecture:

**Terrain system:**
- Infinite procedural terrain via chunk-based streaming
  (`TerrainManager`)
- Each `Chunk` is a 32x32 grid at 2.5 unit resolution, generated from
  Perlin noise heightmap
- Chunks load/unload based on player position (VIEW_DISTANCE = 25 chunks)
- Frustum culling skips rendering chunks outside camera view
- `WorldQuery` trait allows player physics to query terrain height via
  bilinear interpolation of heightmap

**Mesh generation:**
- Heightmap → vertices/indices → raylib `Mesh` via unsafe FFI
- Memory allocated with `libc::malloc`, uploaded to GPU with
  `UploadMesh`
- Vertex colors based on height (darker green → lighter as height
  increases)

**Rendering:**
- Distance fog shader (`client/shaders/fog.{vs,fs}`) applied to all chunk
  models
- Shader uniforms (camera position, fog distances, fog color) updated per
  frame via unsafe `SetShaderValue` FFI calls
- Fog distances calculated as percentage of max view distance
  (fog_near = 40%, fog_far = 50%)

**Player controller:**
- First-person camera with mouse look (yaw/pitch) and WASD movement
- Physics: gravity, jumping, ground detection with terrain snapping
- GOD_MODE constant bypasses physics for free flight
- Sprint (Left Shift) and crouch (Left Control) modifiers

### Message Types

All message enums in `shared/` use `#[serde(tag = "type")]` which
serializes as:

```json
{"type": "PositionUpdate", "position": {"x": 1.0, "y": 2.0, "z": 3.0}}
```

The `type` field acts as the enum discriminant in JSON.

### Known TODOs

- Server does not yet update `Client.player.position` in registry when
  receiving position updates (server/src/main.rs:111)
- If messages disappear, check `try_send` Result handling
  (server/src/main.rs:104)
