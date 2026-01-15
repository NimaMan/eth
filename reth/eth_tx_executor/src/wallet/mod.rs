//! Wallet Module
//! 
//! Manages private keys and transaction signing
//! with security as the top priority.

pub mod secure_wallet;

pub use secure_wallet::{SecureWallet, SecureWalletConfig, SecureWalletError, read_password};