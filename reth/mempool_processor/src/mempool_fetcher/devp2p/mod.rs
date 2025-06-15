/// DevP2P Direct Protocol Method
/// 
/// Implements Ethereum's peer-to-peer protocol for direct transaction
/// propagation without going through RPC/WebSocket layers.
/// 
/// Performance target:
/// - <10ms detection latency
/// - Direct protocol access
/// - Currently 60% implemented

pub mod client;
pub mod protocol;

pub use client::*;
pub use protocol::*;