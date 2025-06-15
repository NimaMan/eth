/// WebSocket Method for Transaction Detection
/// 
/// Uses WebSocket connection to Reth node for real-time transaction streaming.
/// Provides full mempool coverage with reasonable latency.
/// 
/// Performance characteristics:
/// - Average latency: 1-50ms
/// - Full mempool coverage (100%)
/// - More consistent than IPC for subscriptions
/// - Currently used in production

pub mod client;

pub use client::*;