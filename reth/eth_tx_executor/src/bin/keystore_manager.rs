//! Keystore Manager - Create and manage encrypted keystores
//!
//! This utility helps create secure keystores from private keys

use clap::{Parser, Subcommand};
use eth_kartal::wallet::{SecureWallet, SecureWalletConfig, read_password};
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(author, version, about = "ETH Kartal Keystore Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new keystore from a private key
    Create {
        /// Output path for the keystore file
        #[arg(short, long)]
        output: PathBuf,
        
        /// Chain ID (1 for mainnet)
        #[arg(short, long, default_value = "1")]
        chain_id: u64,
        
        /// Private key (will be prompted if not provided)
        #[arg(short, long, env = "ETH_PRIVATE_KEY")]
        private_key: Option<String>,
    },
    
    /// Show address from keystore
    Show {
        /// Path to keystore file
        #[arg(short, long)]
        keystore: PathBuf,
    },
    
    /// Test keystore by signing a message
    Test {
        /// Path to keystore file
        #[arg(short, long)]
        keystore: PathBuf,
        
        /// Chain ID
        #[arg(short, long, default_value = "1")]
        chain_id: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("eth_kartal=info".parse()?))
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Create { output, chain_id, private_key } => {
            create_keystore(output, chain_id, private_key).await?;
        }
        Commands::Show { keystore } => {
            show_keystore(keystore).await?;
        }
        Commands::Test { keystore, chain_id } => {
            test_keystore(keystore, chain_id).await?;
        }
    }
    
    Ok(())
}

async fn create_keystore(
    output: PathBuf,
    chain_id: u64,
    private_key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get private key
    let private_key = match private_key {
        Some(key) => key,
        None => {
            // Read private key securely
            rpassword::prompt_password("Enter private key: ")?
        }
    };
    
    // Get password for encryption
    let password = read_password("Enter password for keystore: ")?;
    let password_confirm = read_password("Confirm password: ")?;
    
    if password.expose_secret() != password_confirm.expose_secret() {
        return Err("Passwords do not match".into());
    }
    
    // Create keystore
    info!("Creating keystore...");
    let wallet = SecureWallet::create_keystore(
        &private_key,
        password,
        &output,
        chain_id,
    ).await?;
    
    info!("Keystore created successfully!");
    info!("Address: {}", wallet.address());
    info!("File: {}", output.display());
    info!("");
    info!("IMPORTANT: Keep this keystore file safe and remember your password!");
    info!("The private key has been encrypted and the original has been cleared from memory.");
    
    Ok(())
}

async fn show_keystore(keystore_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = SecureWalletConfig {
        keystore_path,
        chain_id: 1, // Doesn't matter for showing address
    };
    
    let wallet = SecureWallet::from_keystore(config).await?;
    info!("Keystore address: {}", wallet.address());
    
    Ok(())
}

async fn test_keystore(
    keystore_path: PathBuf,
    chain_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = SecureWalletConfig {
        keystore_path: keystore_path.clone(),
        chain_id,
    };
    
    let wallet = SecureWallet::from_keystore(config).await?;
    info!("Loaded keystore for address: {}", wallet.address());
    
    // Unlock wallet
    let password = read_password("Enter keystore password: ")?;
    wallet.unlock(password).await?;
    info!("Wallet unlocked successfully!");
    
    // Test signing
    use ethers::types::TransactionRequest;
    let test_tx = TransactionRequest::new()
        .to(wallet.address())
        .value(0u64)
        .chain_id(chain_id);
    
    let test_tx: ethers::types::transaction::eip2718::TypedTransaction = test_tx.into();
    let signature = wallet.sign_transaction(&test_tx).await?;
    
    info!("Test signature successful!");
    info!("Signature: 0x{}", hex::encode(signature.to_vec()));
    
    // Lock wallet
    wallet.lock().await;
    info!("Wallet locked");
    
    Ok(())
}

use secrecy::ExposeSecret;