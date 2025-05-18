/*
* Transaction Execution Module
*
* This module implements a comprehensive system for transaction execution on Ethereum,
* with the following workflow:
* 1. Receive transaction data from alerts
* 2. Simulate transaction execution using REVM
* 3. If simulation is successful, execute the transaction
* 4. Monitor transaction status until confirmed or failed
* 5. Report results
*
* The module prioritizes reliability, with simulation-first approach to ensure
* transactions will succeed before committing them to the network.
*/

mod simulator;
mod executor;
mod dispatcher;
mod monitor;
mod status_cache;
mod engine;

// Re-export the main components
pub use engine::TxExecutionEngine;
pub use simulator::TxSimulator;
pub use executor::TxExecutor;
pub use monitor::TxMonitor;

// Public types
pub mod types;

// Error types
pub mod error;

// Configuration
pub mod config;

// Tests
#[cfg(test)]
mod tests; 