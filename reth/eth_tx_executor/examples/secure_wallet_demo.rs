//! Secure Wallet Demo
//! 
//! This example demonstrates how to use the secure wallet functionality
//! to create a keystore, unlock it, and execute a transaction.

use eth_kartal::wallet::{SecureWallet, SecureWalletConfig};
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use secrecy::Secret;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ETH Kartal Secure Wallet Demo ===\n");
    
    // Create a temporary directory for the demo
    let temp_dir = TempDir::new()?;
    let keystore_path = temp_dir.path().join("demo-keystore.json");
    
    // Step 1: Create a new wallet and keystore
    println!("Step 1: Creating a new wallet and encrypted keystore...");
    
    // Generate a random wallet for demo
    let demo_wallet = LocalWallet::new(&mut rand::thread_rng());
    let private_key = hex::encode(demo_wallet.signer().to_bytes());
    let demo_password = Secret::new("demo-password-123".to_string());
    
    // Create secure keystore
    let wallet = SecureWallet::create_keystore(
        &private_key,
        demo_password.clone(),
        &keystore_path,
        1, // mainnet chain ID
    ).await?;
    
    println!("✓ Keystore created at: {}", keystore_path.display());
    println!("✓ Wallet address: {}", wallet.address());
    
    // Step 2: Lock the wallet
    println!("\nStep 2: Locking the wallet (clearing from memory)...");
    wallet.lock().await;
    println!("✓ Wallet locked");
    
    // Step 3: Load wallet from keystore
    println!("\nStep 3: Loading wallet from keystore file...");
    let wallet_config = SecureWalletConfig {
        keystore_path: keystore_path.clone(),
        chain_id: 1,
        auto_lock_timeout: Some(std::time::Duration::from_secs(300)), // 5 minutes
    };
    
    let loaded_wallet = SecureWallet::from_keystore(wallet_config).await?;
    println!("✓ Wallet loaded, address: {}", loaded_wallet.address());
    
    // Step 4: Try to sign without unlocking (should fail)
    println!("\nStep 4: Attempting to sign transaction while locked...");
    let test_tx = TransactionRequest::new()
        .to(Address::random())
        .value(1000u64)
        .chain_id(1u64);
    
    let test_tx: TypedTransaction = test_tx.into();
    
    match loaded_wallet.sign_transaction(&test_tx).await {
        Err(e) => println!("✓ Expected error: {}", e),
        Ok(_) => panic!("Should not be able to sign while locked!"),
    }
    
    // Step 5: Unlock with wrong password (should fail)
    println!("\nStep 5: Attempting to unlock with wrong password...");
    let wrong_password = Secret::new("wrong-password".to_string());
    
    match loaded_wallet.unlock(wrong_password).await {
        Err(_) => println!("✓ Failed to unlock with wrong password (as expected)"),
        Ok(_) => panic!("Should not unlock with wrong password!"),
    }
    
    // Step 6: Unlock with correct password
    println!("\nStep 6: Unlocking wallet with correct password...");
    loaded_wallet.unlock(demo_password).await?;
    println!("✓ Wallet unlocked successfully");
    
    // Step 7: Sign transaction
    println!("\nStep 7: Signing a test transaction...");
    let signature = loaded_wallet.sign_transaction(&test_tx).await?;
    println!("✓ Transaction signed successfully");
    println!("  Signature: 0x{}", hex::encode(signature.to_vec()));
    
    // Step 8: Verify signature
    println!("\nStep 8: Verifying signature...");
    let recovered_address = signature.recover(test_tx.sighash())?;
    assert_eq!(recovered_address, loaded_wallet.address());
    println!("✓ Signature verified! Recovered address matches wallet address");
    
    // Step 9: Lock wallet again
    println!("\nStep 9: Locking wallet to clear keys from memory...");
    loaded_wallet.lock().await;
    println!("✓ Wallet locked");
    
    println!("\n=== Demo completed successfully! ===");
    println!("\nKey features demonstrated:");
    println!("- Encrypted keystore creation");
    println!("- Password protection");
    println!("- Memory clearing when locked");
    println!("- Transaction signing");
    println!("- Signature verification");
    
    Ok(())
}