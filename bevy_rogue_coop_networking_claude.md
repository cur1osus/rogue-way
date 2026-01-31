# Bevy 2D Roguelike Co‑op Networking (Rust) — Implementation Guide for Claude Code

> **Context / tone:** Most of the gameplay is already coming together. What’s missing (or only partially implemented) is the **real co‑op layer**: transport, protocol, server authority, replication, and the glue that keeps Bevy ECS sane across client/server. The goal is to implement this cleanly with minimal churn.

## Goal

Implement **authoritative server + clients** networking for a **2D roguelike** (Vampire Survivors / RoR2‑like with items), up to **4 players**, using **Bevy**.

- **Authoritative**: server simulates world; clients send inputs; server sends deltas + events.
- **Transport**: **QUIC** via `quinn` + async runtime `tokio`.
- **Replication**: per‑client interest management (send only what’s near the player).
- **Events**: inventory changes, pickups, level ups, special spawns are reliable and idempotent.

Deliver this as a small, isolated networking subsystem that can be evolved without rewriting gameplay.

---

## Non‑Goals (avoid scope creep)

- No P2P / NAT traversal / relay in this iteration.
- No anti‑cheat beyond server authority and token checks.
- No full rewind/rollback netcode.
- No “replicate every ECS component automatically” framework; start explicit and pragmatic.

---

## High‑level Architecture

### Binaries
- `game_client` (Bevy app with rendering, input, interpolation)
- `game_server` (headless Bevy app, authoritative simulation)

### Crates / Modules (recommended)
- `net_transport/` — QUIC session, io tasks, reconnect basics
- `net_protocol/` — message types, serialization, versioning
- `net_sync/` — Bevy systems: input capture, delta collection, delta apply, event apply
- `net_world/` — NetId mapping and replicated component representations

If this is a single crate, still keep these as module folders to avoid entanglement.

---

## Tech Stack

Add / ensure dependencies:

```toml
[dependencies]
bevy = "0.14"              # or your current Bevy version
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
quinn = "0.11"
serde = { version = "1", features = ["derive"] }
postcard = { version = "1", features = ["alloc"] }
bytes = "1"
dashmap = "6"
thiserror = "1"
``

> Keep serialization **compact**. Prefer quantized ints for positions/velocities in net updates.

---

## Core Design Decisions

### Tick model
- Server runs fixed step: **30 Hz** (default; configurable).
- Clients render freely; they receive deltas and interpolate remote entities.
- Client sends **InputCmd** at 30 Hz (or every frame but rate-limited), includes `client_seq`.

### Entity identity
- Introduce stable `NetId(u32)` for every replicated entity.
- Maintain mappings:
  - Server: `NetId -> Entity`, `Entity -> NetId`
  - Client: `NetId -> Entity`, `Entity -> NetId`

### Replication categories
Define three replication tiers to avoid bandwidth explosions:
1. **Players (4)** — high frequency (20–30 Hz)
2. **Important** (boss/elites/chests/rare projectiles) — medium frequency
3. **Swarm** (common mobs, common bullets) — low frequency + interest filtering

### Interest management
Per client:
- Compute view radius around player (e.g., 50–80 tiles; tune).
- Only replicate entities whose position is within radius.
- Deltas are computed per client (do not broadcast global deltas blindly).

### Event model
- Anything that must not be dropped/duplicated goes as `Event { event_id, ... }` (reliable stream).
- Events must be **idempotent**: client ignores already-applied `event_id`.
- Inventory/item stacks updates are event-driven, plus periodic sanity checksum optional later.

---

## Protocol (Message Types)

Create protocol enums (exact names ok to differ, but keep semantics):

```rust
#[derive(Serialize, Deserialize)]
pub enum C2S {
    Hello { build: u32, protocol: u16 },
    Join  { lobby: [u8; 16], name: String, token: Option<[u8; 32]> },
    Input(InputCmd),
    Ack   { last_server_tick: u32 },
    Ping  { t: u64 },
}

#[derive(Serialize, Deserialize)]
pub enum S2C {
    HelloOk { tick_hz: u16, player_id: u8, protocol: u16 },
    JoinOk  { world_seed: u64, start_tick: u32, snapshot: Snapshot },
    Delta   { server_tick: u32, base_tick: u32, delta: WorldDelta },
    Event   { server_tick: u32, event_id: u64, kind: EventKind },
    Pong    { t: u64 },
    Kick    { reason: String },
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct InputCmd {
    pub client_seq: u32,
    pub client_tick: u32,
    pub buttons: u16, // dash, interact, skill1..
    pub move_x: i8,
    pub move_y: i8,
    pub aim_x: i8,
    pub aim_y: i8,
}
```

### Snapshot / Delta formats

Minimum viable:

```rust
pub type NetId = u32;

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub players: Vec<PlayerStateNet>,
    pub entities: Vec<SpawnNet>,
}

#[derive(Serialize, Deserialize)]
pub struct WorldDelta {
    pub spawns: Vec<SpawnNet>,
    pub despawns: Vec<NetId>,
    pub updates: Vec<UpdateNet>,
}

#[derive(Serialize, Deserialize)]
pub struct SpawnNet {
    pub id: NetId,
    pub kind: EntityKind,
    pub pos: Vec2i16,    // quantized position
    pub vel: Vec2i16,    // optional
    pub hp: Option<u16>, // optional
}

#[derive(Serialize, Deserialize)]
pub struct UpdateNet {
    pub id: NetId,
    pub flags: u16,      // bitmask of which fields present
    pub pos: Option<Vec2i16>,
    pub vel: Option<Vec2i16>,
    pub hp: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Vec2i16 { pub x: i16, pub y: i16 }
```

Quantization rule example:
- world units are `f32` meters
- net position uses `i16` as `(pos * 32.0)` → ~3cm precision, range ~1024m; adjust scale/range.

---

## Transport (QUIC) Implementation

### Server
- Bind endpoint (configurable host/port).
- Accept incoming connections.
- For each connection:
  - create one **reliable bi-stream** for control/events
  - create one **unidirectional or datagram channel** for frequent inputs (optional)
- Deserialize messages with `postcard` framing:
  - QUIC streams already frame by read boundaries; still implement length-prefix to be safe.

### Client
- Connect to server endpoint.
- Open control stream; send Hello/Join.
- Spawn task to read S2C and forward into Bevy via `mpsc` channel.

### Bevy integration
Use resources:
- `NetInRx` (receiver of messages from transport task)
- `NetOutTx` (sender to transport task)
- `ConnectionState` (Disconnected/Connecting/Connected + player_id)

Transport tasks must never touch Bevy ECS directly.

---

## Bevy Systems (Server)

Implement as a `NetServerPlugin`.

### Resources
- `ServerTick { tick_hz, tick: u32 }`
- `ClientSessions` (DashMap or Vec<Session> indexed by conn id)
- `InputBuffer` per player: ring buffer by `client_seq`/tick
- `NetIdAllocator`

### Systems (order matters)
1. **`server_read_net_in`**
   - drain `C2S` messages from channel
   - validate protocol/build
   - map connection to `player_id`
   - push InputCmd into that player’s `InputBuffer`

2. **`server_fixed_step_sim`** (FixedUpdate at 30 Hz)
   - increment `ServerTick.tick`
   - for each player: pick latest input (or last known) and apply to movement/actions
   - run your normal gameplay simulation (already exists)

3. **`server_collect_deltas_per_client`**
   - for each session:
     - compute interest radius around that client’s player Transform
     - build `WorldDelta`:
       - spawns for newly relevant entities
       - updates for changed entities (pos/vel/hp)
       - despawns for entities that became irrelevant or died
   - store per-client last sent states (baseline) to diff efficiently

4. **`server_send_out`**
   - send `S2C::Delta` over control stream (or separate stream) via transport channel
   - send `S2C::Event` reliably when gameplay events occur (see below)

### Event emission on server
Create an event queue resource:
- gameplay systems push `ServerGameEvent` (picked up item, leveled up, chest opened, etc.)
- networking layer converts to `S2C::Event { event_id, kind }`
- `event_id` is monotonic per server (u64), or per client.

---

## Bevy Systems (Client)

Implement as `NetClientPlugin`.

### Resources
- `ClientTick` (optional)
- `RemoteInterpBuffer` per entity (store last 2 snapshots)
- `PendingEvents` set (applied event_id LRU)

### Systems
1. **`client_capture_input`**
   - sample input axes/buttons
   - build InputCmd with `client_seq += 1`
   - send to server at 30 Hz (rate limit)

2. **`client_read_net_in`**
   - apply `JoinOk` snapshot:
     - spawn initial entities
     - map NetIds
   - apply `Delta`:
     - spawns/despawns
     - updates → add to interp buffers
   - apply `Event` idempotently:
     - inventory changes, UI notifications, etc.

3. **`client_interpolate_remote`**
   - for non-local players and swarm entities:
     - interpolate between last two received states
   - for local player:
     - either authoritative snap with gentle correction OR simple prediction + reconciliation (optional)

> Start without sophisticated prediction. If movement feels laggy, add local prediction only for the local player.

---

## Acceptance Criteria

You’re done when:

1. **Join flow works**
   - client connects, sends Hello/Join, receives snapshot, enters play state.

2. **4 players can move and shoot**
   - server authoritative; clients see each other smoothly (interpolation).

3. **Bandwidth stays sane**
   - swarm entities are interest-filtered, deltas are compact (quantized).

4. **Inventory items replicate correctly**
   - picking up an item triggers reliable event; stacks correct; no dupes on reconnect.

5. **Deterministic-ish server state**
   - clients do not author core game state (damage, loot, leveling).

6. **Disconnect/reconnect doesn’t crash**
   - server cleans session; client handles Kick/Disconnect gracefully.

---

## Testing Plan (must implement)

### Local multi-client
- run server headless
- spawn 2–4 clients locally
- verify join/movement/shooting

### Soak
- 10 minutes of continuous play with heavy swarm
- check memory growth (NetId maps, interp buffers, applied-event LRU)

### Debug overlays (client)
- show RTT
- show server_tick vs client time
- show number of replicated entities currently visible
- show bytes/sec in/out (simple counters)

---

## Implementation Notes / Pitfalls (do not ignore)

- **TCP-like head-of-line**: QUIC streams isolate; keep frequent updates separate from big snapshot transfers.
- **Message framing**: even with QUIC streams, implement length-prefix framing to avoid partial reads issues.
- **Quantization**: never send raw `f32` for swarm; use ints.
- **Baseline diffing**: per-client last-sent state matters; otherwise you’ll spam full updates.
- **Entity churn**: bullets spawn/despawn frequently. Consider:
  - replicate only “important” projectiles
  - or represent common bullets as short-lived events (“spawn bullet with velocity”) if needed later.

---

## “Already implemented” assumptions (adjust to actual project)
These are likely already present and should be reused:
- player movement + combat logic
- spawn systems for mobs/projectiles/pickups
- item system (stacks, procs)

Networking must **wrap** these systems, not rewrite them.

---

## Extra Ideas (optional but valuable)
Implement once core works:

1. **Server-side lag compensation lite**
   - accept slightly delayed inputs for shooting direction within a small window

2. **Compression for join snapshot only**
   - zstd/lz4 on Snapshot payloads

3. **Auth token via matchmaker**
   - later: HTTP matchmaker issues signed join tokens; server verifies HMAC

4. **Replayable event log**
   - store last N events so late joiners can catch up without huge snapshots (later)

---

## Deliverables Checklist (what to commit)

- [ ] `game_server` binary that runs headless and accepts QUIC clients
- [ ] `game_client` networking plugin integrated with Bevy app
- [ ] `net_protocol` message types + postcard serialization
- [ ] `NetId` mapping system server+client
- [ ] Interest management per client
- [ ] Delta replication + idempotent events
- [ ] Minimal debug UI (RTT, replicated entity count, bytes/sec)
- [ ] Local run scripts (`cargo run --bin game_server`, etc.)

---

## Minimal run commands

Server:
```bash
cargo run --bin game_server -- --host 0.0.0.0 --port 25565
```

Client:
```bash
cargo run --bin game_client -- --connect 127.0.0.1:25565 --name Player1
```

---

## Final note
Focus on **clean boundaries**:
- transport async tasks <-> Bevy via channels
- protocol types isolated
- replication explicit and testable

If you feel tempted to “auto-replicate the whole ECS world”, don’t. Start explicit; you can generalize later.
