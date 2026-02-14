//! Alert Processor Module
//!
//! Receives and processes execution alerts via high-performance messaging

pub mod receiver;
pub mod supervised_receiver;
pub mod types;
pub mod validation;

pub use receiver::{AlertReceiver, ReceiverConfig};
pub use types::{Action, Alert, ExecutionParams, Priority};
