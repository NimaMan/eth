/// IPC-IPC Variants (Legacy Implementations)
/// 
/// Multiple IPC-based implementations for connecting to a local Reth node
/// via Unix domain socket. All use IPC for both subscription and transaction fetching.
/// 
/// Performance characteristics:
/// - Full TX client: IPC subscription + IPC individual fetch (1.040ms avg - BEST)
/// 
/// Available implementations:
/// - `FullTxIpcClient`: Gets full transaction data via IPC fetch
/// 
/// NOTE: These are legacy implementations. For new development, use ../ipc_ipc/ instead.
/// Basic, Batch, and Optimized variants have been removed due to poor performance.

pub mod full_tx_client;

pub use full_tx_client::{FullTxIpcClient, FullIpcTransaction};