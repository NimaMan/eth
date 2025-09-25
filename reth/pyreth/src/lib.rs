/// PyReth - Python bindings for Ethereum transaction simulation
///
/// This crate provides Python bindings for tx_simulator functionality.
/// Starting minimal with just simulation, will add other components incrementally.
// Core Rust modules exposed to Python bindings
pub mod agents;
pub mod chain_query;
pub mod price_reader;
pub mod provider;
pub mod pyreth_instance;
pub mod simulator;
pub mod tx_processor;

// Python bindings wrapper
pub mod python;

// Re-export the Python module function for maturin
pub use python::pyreth_module;
