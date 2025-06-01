pub mod conversions;
// pub mod state_diff_utils; // Commented out as the file was removed
pub mod simulation_core;

// Re-export common types for easier use in examples or by other crates
pub use conversions::*;
pub use simulation_core::{simulate_transaction, SimulationOutput, SimCacheDB, ExecutionResultType};

// Conversion functions have been moved to src/conversions.rs

#[cfg(test)]
mod tests {
    // Tests for conversion functions are now in src/conversions.rs
    // If there were other lib-specific tests, they would remain here.
    // For example, a test for hello_from_lib if it existed.
} 