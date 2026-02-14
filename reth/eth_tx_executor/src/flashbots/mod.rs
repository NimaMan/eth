//! Flashbots integration for MEV protection and priority execution
//!
//! Provides direct-to-validator transaction submission for critical alerts

pub mod bundle;
pub mod client;
pub mod signer;
pub mod simulation;
pub mod types;

pub use bundle::{Bundle, BundleBuilder};
pub use client::{FlashbotsClient, FlashbotsConfig, RelayEndpoint};
pub use signer::BundleSigner;
pub use simulation::BundleSimulator;
pub use types::{BundleResult, BundleStatus, SimulationResult};
