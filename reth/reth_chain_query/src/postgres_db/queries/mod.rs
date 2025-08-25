/// Query Modules for PostgreSQL Database
/// 
/// Organized by data domain for efficient access to aggregated Ethereum data.

pub mod address_metrics;
pub mod trades;
pub mod tokens;
pub mod analytics;
pub mod aggregation;
pub mod population;
pub mod transactions;

// Re-export commonly used functions
pub use address_metrics::{get_address_metrics, get_top_profitable_addresses};
pub use trades::{get_trades_for_address, get_address_token_pnl, get_address_trade_summary};
pub use tokens::{get_token_info, get_token_pools, get_scam_tokens};
pub use analytics::{calculate_address_ranking, get_network_relationships};
pub use aggregation::{aggregate_address_metrics, calculate_scam_ratios};
pub use population::{populate_addresses_from_trades, get_population_statistics};
pub use transactions::{
    get_address_transactions, get_tx_participants, get_address_id,
    count_address_transactions, get_transaction_with_participants
};