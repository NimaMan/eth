//! Alert Processor Module
//! 
//! Receives and processes execution alerts via high-performance messaging

pub mod receiver;
pub mod types;

pub use receiver::{AlertReceiver, ReceiverConfig};
pub use types::{Alert, Action, ExecutionParams, Priority};