//! Wallet Module
//! 
//! Manages private keys, transaction signing, and account balances
//! with security as the top priority.

pub mod position_tracker;
pub mod secure_wallet;

pub use position_tracker::{PositionTracker, TokenPosition};
pub use secure_wallet::{SecureWallet, SecureWalletConfig, SecureWalletError, read_password};