pub mod delta;
pub mod messages;
pub mod quantization;
pub mod snapshots;

// Re-export commonly used types
pub use delta::{UpdateNet, WorldDelta, WorldDeltaBuilder};
pub use messages::{
    C2S, EventKind, FxEventData, InputCmd, ItemUpdate, S2C, MAX_MESSAGE_SIZE, PROTOCOL_VERSION,
};
pub use quantization::{Dequantize, Quantize, Vec2i16, QUANTIZATION_SCALE};
pub use snapshots::{
    convert_legacy_snapshot_to_net, EnemyStateNet, EntityKind, GoldStateNet, NetId,
    PetStateNet, PlayerStateNet, ProjectileStateNet, Snapshot, SpawnNet, XpGemStateNet,
};
