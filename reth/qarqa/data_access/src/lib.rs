//! QARQA Data Access Layer
//!
//! Handles all database connections, queries, and data fetching operations.
//! Uses the participants table for O(1) address transaction lookups.

pub mod database;
pub mod address_fetcher;
pub mod transaction_fetcher;
pub mod block_fetcher;

pub use database::*;
pub use address_fetcher::*;
pub use transaction_fetcher::*;
pub use block_fetcher::*;