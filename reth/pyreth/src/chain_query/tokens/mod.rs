//! Token-related Python bindings for ChainQuery
//!
//! Objective:
//! - Provide a clean home for token metadata helpers and future token-centric queries.
//! - Keep `chain_query.rs` focused on accounts/balances/portfolios.
//!
//! Exposed (via pyreth.ChainQuery):
//! - get_token_decimals(token, block=None) -> int
//! - get_token_symbol(token, block=None) -> str
//! - get_token_name(token, block=None) -> str
//! - get_token_total_supply(token, block=None) -> str
//! - get_token_metadata(token, block=None) -> Optional[TokenMetadata]

use pyo3::prelude::*;

/// Python wrapper for Token metadata
#[pyclass(name = "TokenMetadata")]
#[derive(Clone)]
pub struct PyTokenMetadata {
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub symbol: String,
    #[pyo3(get)]
    pub decimals: u8,
    #[pyo3(get)]
    pub total_supply: String,
}
