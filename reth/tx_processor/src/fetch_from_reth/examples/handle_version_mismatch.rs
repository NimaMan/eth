//! Example: Handling MDBX Version Mismatch Errors
//!
//! This example demonstrates how to handle MDBX version mismatch errors
//! when opening a Reth database with the fetch_from_reth module.

use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig, 
    CompatibilityMode, FetchError, read_database_version,
};
use std::path::Path;
use tracing::{info, warn, error};
use tracing_subscriber;

fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Get Reth datadir from environment or use default
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    info!("Attempting to open Reth database at: {}", datadir);
    
    // First, try to read database version information
    match read_database_version(Path::new(&datadir)) {
        Ok(version_info) => {
            info!("Database version info: {:?}", version_info);
        }
        Err(e) => {
            warn!("Could not read database version: {}", e);
        }
    }
    
    // Example 1: Try with default (Auto) mode
    info!("\n=== Example 1: Auto Mode (Default) ===");
    match RethDatabaseProvider::new(&datadir) {
        Ok(provider) => {
            info!("Successfully opened database with auto mode!");
            test_provider(&provider)?;
        }
        Err(e) => {
            error!("Failed with auto mode: {}", e);
            
            // Check if it's a version mismatch
            if let FetchError::VersionMismatch { expected, actual, code } = &e {
                info!("Version mismatch detected:");
                info!("  Expected: {}", expected);
                info!("  Actual: {}", actual);
                info!("  Error code: {:?}", code);
            }
        }
    }
    
    // Example 2: Try with Compatibility mode
    info!("\n=== Example 2: Compatibility Mode ===");
    let config = RethDataConfig::new(&datadir)
        .with_compatibility_mode(CompatibilityMode::Compatible);
    
    match RethDatabaseProvider::with_config(config) {
        Ok(provider) => {
            info!("Successfully opened database with compatibility mode!");
            test_provider(&provider)?;
        }
        Err(e) => {
            error!("Failed with compatibility mode: {}", e);
        }
    }
    
    // Example 3: Force open mode (use with caution!)
    info!("\n=== Example 3: Force Open Mode ===");
    warn!("Force open mode can lead to inconsistent reads - use only for debugging!");
    
    let config = RethDataConfig::new(&datadir)
        .with_compatibility_mode(CompatibilityMode::Force)
        .with_force_open(true);
    
    match RethDatabaseProvider::with_config(config) {
        Ok(provider) => {
            info!("Successfully opened database with force mode!");
            test_provider(&provider)?;
        }
        Err(e) => {
            error!("Failed even with force mode: {}", e);
            error!("This likely means:");
            error!("1. The database is actively being used by Reth");
            error!("2. The database path is incorrect");
            error!("3. Permission issues preventing access");
        }
    }
    
    // Example 4: Handling errors gracefully
    info!("\n=== Example 4: Error Handling Pattern ===");
    let result = open_database_with_fallback(&datadir);
    match result {
        Ok(provider) => {
            info!("Successfully opened database with fallback strategy!");
            test_provider(&provider)?;
        }
        Err(e) => {
            error!("All strategies failed: {}", e);
            suggest_solutions(&e);
        }
    }
    
    Ok(())
}

/// Test the provider by fetching some data
fn test_provider(provider: &RethDatabaseProvider) -> eyre::Result<()> {
    // Try to get latest block number
    match provider.latest_block_number() {
        Ok(block_num) => {
            info!("Latest block number: {}", block_num);
        }
        Err(e) => {
            error!("Failed to get latest block: {}", e);
        }
    }
    
    // Try to fetch a known transaction (USDC creation)
    let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"
        .parse()
        .unwrap();
    
    match provider.fetch_transaction(tx_hash) {
        Ok(tx_data) => {
            info!("Successfully fetched transaction:");
            info!("  From: {}", tx_data.from);
            info!("  Block: {}", tx_data.block_number);
        }
        Err(e) => {
            warn!("Could not fetch transaction: {}", e);
        }
    }
    
    Ok(())
}

/// Example of a robust database opening strategy with fallbacks
fn open_database_with_fallback(datadir: &str) -> Result<RethDatabaseProvider, FetchError> {
    // Strategy 1: Try normal mode
    if let Ok(provider) = RethDatabaseProvider::new(datadir) {
        info!("Opened database in normal mode");
        return Ok(provider);
    }
    
    // Strategy 2: Try compatibility mode
    let compat_config = RethDataConfig::new(datadir)
        .with_compatibility_mode(CompatibilityMode::Compatible)
        .with_read_only(true); // Skip consistency check not available
    
    if let Ok(provider) = RethDatabaseProvider::with_config(compat_config) {
        info!("Opened database in compatibility mode");
        return Ok(provider);
    }
    
    // Strategy 3: Last resort - force mode with warnings
    warn!("Attempting force mode as last resort...");
    let force_config = RethDataConfig::new(datadir)
        .with_compatibility_mode(CompatibilityMode::Force)
        .with_force_open(true)
        .with_read_only(true);
    
    RethDatabaseProvider::with_config(force_config)
}

/// Suggest solutions based on the error type
fn suggest_solutions(error: &FetchError) {
    info!("\n=== Troubleshooting Guide ===");
    
    match error {
        FetchError::VersionMismatch { .. } => {
            info!("Solution for version mismatch:");
            info!("1. Stop Reth if it's running: `killall reth`");
            info!("2. Check Reth version: `reth --version`");
            info!("3. Update to matching version or rebuild database");
            info!("4. As last resort, try removing lock file:");
            info!("   rm ~/.local/share/reth/mainnet/db/mdbx.lck");
        }
        FetchError::ConfigError(msg) if msg.contains("does not exist") => {
            info!("Database path not found. Check:");
            info!("1. Is Reth installed and initialized?");
            info!("2. Correct path? Default: ~/.local/share/reth/mainnet");
            info!("3. Set RETH_DATADIR environment variable");
        }
        FetchError::IoError(msg) if msg.contains("Permission") => {
            info!("Permission issue detected:");
            info!("1. Check file ownership: `ls -la ~/.local/share/reth`");
            info!("2. Ensure your user has read access");
            info!("3. If using Docker, check volume permissions");
        }
        _ => {
            info!("General troubleshooting:");
            info!("1. Check Reth logs: `journalctl -u reth -n 100`");
            info!("2. Verify disk space: `df -h`");
            info!("3. Check system resources: `htop`");
        }
    }
}