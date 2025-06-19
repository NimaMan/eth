//! Alert Processor Module
//! 
//! Receives and processes scam detection alerts from the mempool processor
//! via ZeroMQ messaging.

pub mod receiver;
pub mod types;

pub use receiver::{AlertReceiver, ReceiverConfig};
pub use types::{AlertMessage, ScamAlert, Severity, EventType};