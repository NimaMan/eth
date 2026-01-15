//! Address-related helpers for ChainQuery
//!
//! Intended to group address/EOA/contract-centric methods and types.
//! Current methods remain implemented in `chain_query.rs`:
//! - get_account, get_nonce, is_contract, batch_is_contract
//! - get_eth_balance, batch_get_eth_balances
//!
//! This module serves as a namespace for future refactors to keep the
//! bindings tidy and logically organized.
