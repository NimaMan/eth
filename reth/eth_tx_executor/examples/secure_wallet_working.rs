//! Working Secure Wallet Example
//! 
//! This example demonstrates secure key management in practice

use eth_kartal::wallet::{SecureWallet, SecureWalletConfig, read_password};
use ethers::prelude::*;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ETH Kartal Secure Wallet Working Example ===\n");
    
    // Get keystore path from environment or use default
    let keystore_path = env::var("ETH_KEYSTORE_PATH")
        .unwrap_or_else(|_| "./example-keystore.json".to_string());
    let keystore_path = PathBuf::from(keystore_path);
    
    println!("Using keystore at: {}", keystore_path.display());
    
    // Check if keystore exists
    if !keystore_path.exists() {
        println!("\nKeystore not found. Let's create one!");
        println!("Please run the keystore_manager to create a keystore first:");
        println!("\n  cargo run --bin keystore_manager create -o {}", keystore_path.display());
        println!("\nThen run this example again.");
        return Ok(());
    }
    
    // Load wallet from keystore
    println!("\nLoading wallet from keystore...");
    let config = SecureWalletConfig {
        keystore_path: keystore_path.clone(),
        chain_id: 1, // mainnet
        auto_lock_timeout: Some(std::time::Duration::from_secs(300)), // 5 minutes
    };
    
    let wallet = SecureWallet::from_keystore(config).await?;
    println!("✓ Wallet loaded successfully");
    println!("  Address: {}", wallet.address());
    
    // Check if already unlocked
    if wallet.is_unlocked().await {
        println!("✓ Wallet is already unlocked");
    } else {
        // Prompt for password
        let password = read_password("Enter keystore password: ")?;
        
        // Unlock wallet
        println!("Unlocking wallet...");
        match wallet.unlock(password).await {
            Ok(_) => println!("✓ Wallet unlocked successfully"),
            Err(e) => {
                println!("✗ Failed to unlock wallet: {}", e);
                return Err(e.into());
            }
        }
    }
    
    // Demonstrate signing
    println!("\nDemonstrating transaction signing...");
    
    // Create a test transaction
    let test_tx = TransactionRequest::new()
        .to("0x0000000000000000000000000000000000000000".parse::<Address>()?)
        .value(0u64)
        .gas(21000u64)
        .gas_price(20_000_000_000u64) // 20 gwei
        .nonce(0u64)
        .chain_id(1u64);
    
    let test_tx: ethers::types::transaction::eip2718::TypedTransaction = test_tx.into();
    
    // Sign transaction
    println!("Signing transaction...");
    let signature = wallet.sign_transaction(&test_tx).await?;
    println!("✓ Transaction signed successfully");
    println!("  Signature: 0x{}", hex::encode(signature.to_vec()));
    
    // Verify signature
    let sighash = test_tx.sighash();
    let recovered_address = signature.recover(sighash)?;
    println!("✓ Signature verified");
    println!("  Recovered address: {}", recovered_address);
    println!("  Matches wallet: {}", recovered_address == wallet.address());
    
    // Lock wallet
    println!("\nLocking wallet...");
    wallet.lock().await;
    println!("✓ Wallet locked (keys cleared from memory)");
    
    // Try to sign after locking (should fail)
    println!("\nTrying to sign after locking...");
    match wallet.sign_transaction(&test_tx).await {
        Err(_) => println!("✓ Correctly refused to sign while locked"),
        Ok(_) => println!("✗ ERROR: Should not be able to sign while locked!"),
    }
    
    println!("\n=== Example completed successfully! ===");
    
    Ok(())
}