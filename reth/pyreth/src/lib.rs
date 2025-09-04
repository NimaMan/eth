/// PyReth - Python bindings for Ethereum transaction simulation
/// 
/// This crate provides Python bindings for tx_simulator functionality.
/// Starting minimal with just simulation, will add other components incrementally.

// Python bindings
pub mod python;

// Re-export the Python module function for maturin
pub use python::pyreth_module;