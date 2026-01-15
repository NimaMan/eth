/// Query Modules for PostgreSQL Database
///
/// Organized by data domain for efficient access to aggregated Ethereum data.
pub mod address_metrics;
pub mod aggregation;
pub mod analytics;
pub mod population;
pub mod tokens;
pub mod trades;
pub mod transactions;

// Re-export commonly used functions
pub use address_metrics::{get_address_metrics, get_top_profitable_addresses};
pub use aggregation::{aggregate_address_metrics, calculate_scam_ratios};
pub use analytics::{calculate_address_ranking, get_network_relationships};
pub use population::{get_population_statistics, populate_addresses_from_trades};
pub use tokens::{get_scam_tokens, get_token_info, get_token_pools};
pub use trades::{get_address_token_pnl, get_address_trade_summary, get_trades_for_address};
pub use transactions::{
    count_address_transactions, get_address_id, get_address_transactions,
    get_transaction_with_participants, get_tx_participants,
};
