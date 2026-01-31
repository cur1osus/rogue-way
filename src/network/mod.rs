// Temporary: re-export legacy networking for compatibility
pub mod legacy;

// Re-export legacy types for backward compatibility
pub use legacy::{
    is_authoritative, is_client, is_host, start_client, start_host, NetFxEvent, NetInput,
    NetMessage, NetUdpMessage, NetworkClient, NetworkEntityMap, NetworkId,
    NetworkIdAllocator, NetworkInterpolation, NetworkMode, NetworkPlugin, NetworkServer,
    DEFAULT_PORT, LOCAL_PLAYER_ID,
};

// New modular structure
pub mod protocol;
pub mod transport;
pub mod world;
pub mod sync;
pub mod quic_integration;
pub mod debug_ui;
