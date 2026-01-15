//! Secure wallet implementation with encrypted keystore support
//!
//! This module provides secure key management using standard Ethereum keystore format
//! with password protection and automatic memory clearing.

use eth_keystore::{decrypt_key, encrypt_key, KeystoreError};
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::transaction::eip712::Eip712;
use secrecy::{ExposeSecret, Secret, Zeroize};
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, error, debug};

/// Secure wallet configuration
#[derive(Debug, Clone)]
pub struct SecureWalletConfig {
    /// Path to keystore file
    pub keystore_path: PathBuf,
    /// Chain ID
    pub chain_id: u64,
    /// Auto-lock timeout (None means no auto-lock)
    pub auto_lock_timeout: Option<Duration>,
}

/// Errors that can occur with secure wallet
#[derive(Debug, thiserror::Error)]
pub enum SecureWalletError {
    #[error("Keystore error: {0}")]
    Keystore(#[from] KeystoreError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
    
    #[error("Keystore file not found at: {0}")]
    KeystoreNotFound(String),
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Wallet locked - unlock required")]
    WalletLocked,
}

/// Secure wallet with encrypted keystore
pub struct SecureWallet {
    /// Configuration
    config: SecureWalletConfig,
    /// Currently loaded wallet (if unlocked)
    wallet: Arc<RwLock<Option<LocalWallet>>>,
    /// Wallet address (always available)
    address: Address,
    /// Last activity timestamp
    last_activity: Arc<RwLock<Option<Instant>>>,
    /// Auto-lock task handle
    auto_lock_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl SecureWallet {
    /// Create new secure wallet from keystore file
    pub async fn from_keystore(config: SecureWalletConfig) -> Result<Self, SecureWalletError> {
        // Verify keystore exists
        if !config.keystore_path.exists() {
            return Err(SecureWalletError::KeystoreNotFound(
                config.keystore_path.display().to_string()
            ));
        }
        
        // Read keystore to get address (doesn't require password)
        let keystore_contents = fs::read_to_string(&config.keystore_path)?;
        let keystore: serde_json::Value = serde_json::from_str(&keystore_contents)
            .map_err(|_| SecureWalletError::InvalidPassword)?;
        
        // Extract address from keystore
        let address_str = keystore["address"]
            .as_str()
            .ok_or(SecureWalletError::InvalidPassword)?;
        
        let address = format!("0x{}", address_str)
            .parse::<Address>()
            .map_err(|_| SecureWalletError::InvalidPassword)?;
        
        info!("Loaded secure wallet for address: {}", address);
        
        Ok(Self {
            config,
            wallet: Arc::new(RwLock::new(None)),
            address,
            last_activity: Arc::new(RwLock::new(None)),
            auto_lock_handle: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Create new keystore file with password
    pub async fn create_keystore(
        private_key: &str,
        password: Secret<String>,
        keystore_path: &Path,
        chain_id: u64,
    ) -> Result<Self, SecureWalletError> {
        // Parse private key
        let wallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| SecureWalletError::Wallet(e))?
            .with_chain_id(chain_id);
        
        let address = wallet.address();
        
        // Create keystore directory if needed
        if let Some(parent) = keystore_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Generate random encryption parameters
        let mut rng = rand::thread_rng();
        
        // Encrypt and save keystore - the function signature takes a private key slice
        let private_key_bytes = wallet.signer().to_bytes();
        let _keystore = encrypt_key(
            keystore_path,
            &mut rng,
            &private_key_bytes,
            password.expose_secret(),
            None, // Use default KDF params
        )?;
        
        info!("Created new keystore for address: {} at {}", address, keystore_path.display());
        
        Ok(Self {
            config: SecureWalletConfig {
                keystore_path: keystore_path.to_path_buf(),
                chain_id,
                auto_lock_timeout: None, // Default to no auto-lock
            },
            wallet: Arc::new(RwLock::new(Some(wallet))),
            address,
            last_activity: Arc::new(RwLock::new(Some(Instant::now()))),
            auto_lock_handle: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Unlock wallet with password
    pub async fn unlock(&self, password: Secret<String>) -> Result<(), SecureWalletError> {
        // Decrypt private key from keystore
        let private_key_bytes = decrypt_key(&self.config.keystore_path, password.expose_secret())?;
        
        // Create wallet from decrypted key
        let mut private_key = hex::encode(&private_key_bytes);
        let wallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| SecureWalletError::Wallet(e))?
            .with_chain_id(self.config.chain_id);
        
        // Store wallet
        let mut w = self.wallet.write().await;
        *w = Some(wallet);
        
        // Clear sensitive data
        private_key.zeroize();
        
        // Update last activity
        *self.last_activity.write().await = Some(Instant::now());
        
        // Start auto-lock task if configured
        if let Some(timeout) = self.config.auto_lock_timeout {
            self.start_auto_lock_task(timeout).await;
        }
        
        info!("Wallet unlocked successfully");
        Ok(())
    }
    
    /// Lock wallet (clear from memory)
    pub async fn lock(&self) {
        let mut w = self.wallet.write().await;
        *w = None;
        
        // Clear last activity
        *self.last_activity.write().await = None;
        
        // Cancel auto-lock task
        if let Some(handle) = self.auto_lock_handle.write().await.take() {
            handle.abort();
        }
        
        info!("Wallet locked");
    }
    
    /// Check if wallet is unlocked
    pub async fn is_unlocked(&self) -> bool {
        self.wallet.read().await.is_some()
    }
    
    /// Get wallet address (always available)
    pub fn address(&self) -> Address {
        self.address
    }
    
    /// Sign transaction (requires unlocked wallet)
    pub async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, SecureWalletError> {
        let wallet_guard = self.wallet.read().await;
        let wallet = wallet_guard
            .as_ref()
            .ok_or(SecureWalletError::WalletLocked)?;
        
        // Update last activity
        *self.last_activity.write().await = Some(Instant::now());
        
        wallet.sign_transaction(tx)
            .await
            .map_err(|e| SecureWalletError::Wallet(e))
    }
    
    /// Sign typed data (requires unlocked wallet)
    pub async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, SecureWalletError> {
        let wallet_guard = self.wallet.read().await;
        let wallet = wallet_guard
            .as_ref()
            .ok_or(SecureWalletError::WalletLocked)?;
        
        // Update last activity
        *self.last_activity.write().await = Some(Instant::now());
        
        wallet.sign_typed_data(payload)
            .await
            .map_err(|e| SecureWalletError::Wallet(e))
    }
    
    /// Start auto-lock background task
    async fn start_auto_lock_task(&self, timeout: Duration) {
        // Cancel existing task if any
        if let Some(handle) = self.auto_lock_handle.write().await.take() {
            handle.abort();
        }
        
        let wallet = self.wallet.clone();
        let last_activity = self.last_activity.clone();
        let auto_lock_handle = self.auto_lock_handle.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30)); // Check every 30 seconds
            
            loop {
                interval.tick().await;
                
                // Check if we should auto-lock
                let should_lock = {
                    let last_activity_guard = last_activity.read().await;
                    if let Some(last) = *last_activity_guard {
                        Instant::now().duration_since(last) > timeout
                    } else {
                        false // Already locked
                    }
                };
                
                if should_lock {
                    debug!("Auto-locking wallet due to inactivity");
                    // Lock the wallet
                    *wallet.write().await = None;
                    *last_activity.write().await = None;
                    info!("Wallet auto-locked due to inactivity");
                    
                    // Clear our own handle
                    *auto_lock_handle.write().await = None;
                    break;
                }
            }
        });
        
        *self.auto_lock_handle.write().await = Some(handle);
    }
}

/// Password input helper
pub fn read_password(prompt: &str) -> Result<Secret<String>, std::io::Error> {
    print!("{}", prompt);
    use std::io::{stdout, Write};
    stdout().flush()?;
    
    let password = rpassword::read_password()?;
    Ok(Secret::new(password))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_keystore_creation_and_usage() {
        // Create temp directory for keystore
        let temp_dir = TempDir::new().unwrap();
        let keystore_path = temp_dir.path().join("test-keystore.json");
        
        // Generate random wallet for testing
        let test_wallet = LocalWallet::new(&mut rand::thread_rng());
        let private_key = format!("0x{}", hex::encode(test_wallet.signer().to_bytes()));
        let test_password = Secret::new("test-password-123".to_string());
        
        // Create keystore
        let wallet = SecureWallet::create_keystore(
            &private_key,
            test_password.clone(),
            &keystore_path,
            1, // mainnet
        ).await.unwrap();
        
        // Verify address matches
        assert_eq!(wallet.address(), test_wallet.address());
        
        // Lock and verify locked
        wallet.lock().await;
        assert!(!wallet.is_unlocked().await);
        
        // Try to sign while locked (should fail)
        let tx = TransactionRequest::new()
            .to(Address::random())
            .value(1000u64);
        let tx: TypedTransaction = tx.into();
        
        assert!(matches!(
            wallet.sign_transaction(&tx).await,
            Err(SecureWalletError::WalletLocked)
        ));
        
        // Unlock with correct password
        wallet.unlock(test_password.clone()).await.unwrap();
        assert!(wallet.is_unlocked().await);
        
        // Sign transaction should work now
        let signature = wallet.sign_transaction(&tx).await.unwrap();
        assert!(!signature.to_vec().is_empty());
        
        // Load from existing keystore
        let wallet2 = SecureWallet::from_keystore(SecureWalletConfig {
            keystore_path: keystore_path.clone(),
            chain_id: 1,
        }).await.unwrap();
        
        assert_eq!(wallet2.address(), test_wallet.address());
        
        // Unlock and verify same signature
        wallet2.unlock(test_password).await.unwrap();
        let signature2 = wallet2.sign_transaction(&tx).await.unwrap();
        assert_eq!(signature.to_vec(), signature2.to_vec());
    }
    
    #[tokio::test]
    async fn test_wrong_password() {
        let temp_dir = TempDir::new().unwrap();
        let keystore_path = temp_dir.path().join("test-keystore.json");
        
        let test_wallet = LocalWallet::new(&mut rand::thread_rng());
        let private_key = hex::encode(test_wallet.signer().to_bytes());
        let correct_password = Secret::new("correct-password".to_string());
        let wrong_password = Secret::new("wrong-password".to_string());
        
        // Create keystore
        let wallet = SecureWallet::create_keystore(
            &private_key,
            correct_password,
            &keystore_path,
            1,
        ).await.unwrap();
        
        wallet.lock().await;
        
        // Try to unlock with wrong password
        let result = wallet.unlock(wrong_password).await;
        assert!(result.is_err());
    }
}