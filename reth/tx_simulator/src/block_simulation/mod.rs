/// Block-level simulation module
/// 
/// This module provides functionality to simulate entire blocks of transactions,
/// bypassing RPC calls for `debug_traceBlockByNumber`.
/// 
/// ## Features
/// - Trace all transactions in a block (equivalent to debug_traceBlockByNumber)
/// - Direct database access for 100-1000x performance improvement
/// - Sequential transaction execution with state persistence
/// - Exact RPC equivalence

pub mod types;
pub mod block_tracer;

pub use types::{
    BlockTraceResult,
    TransactionTraceResult,
    BlockSimulationOptions,
};
pub use block_tracer::BlockTracer;