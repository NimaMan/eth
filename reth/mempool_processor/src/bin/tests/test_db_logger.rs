/*
 * Database Logger Test Script
 * 
 * This utility demonstrates how to use the DbLogger to log mempool scam predictions
 * to a PostgreSQL database.
 */

use clap::Parser;
use mempool_processor::mempool_processor::db_logger::DbLogger;
use tracing::{info, error, Level};
use eyre::Result;

#[derive(Parser, Debug)]
struct Args {
    /// Database host
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    /// Database port
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    /// Database name
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    /// Database user
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    /// Database password
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// Token address for test prediction
    #[arg(long, default_value = "0x0000000000000000000000000000000000000001")]
    token_address: String,
    
    /// Pool address for test prediction
    #[arg(long, default_value = "0x0000000000000000000000000000000000000002")]
    pool_address: String,
    
    /// Skip foreign key checks (useful for testing without valid token references)
    #[arg(long)]
    skip_fk_validation: bool,
    
    /// Keep test data in database after test (don't clean up)
    #[arg(long)]
    keep_test_data: bool,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("Database Logger Test Starting");
    info!("Database: {}:{}/{}", args.db_host, args.db_port, args.db_name);
    
    // Initialize the database logger
    info!("Initializing database logger...");
    let db_logger = match DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name,
    ).await {
        Ok(logger) => {
            info!("Database logger initialized successfully");
            logger
        },
        Err(e) => {
            error!("Failed to initialize database logger: {}", e);
            return Err(eyre::eyre!("Database connection error: {}", e));
        }
    };
    
    // Example values for a scam prediction
    let block_number = 15_000_000_i64;
    let current_eth_level = 10.5;
    let simulated_eth_level = 2.0;
    let eth_threshold = 5.0;
    
    // Write a test prediction to the database
    info!("Writing test prediction to database...");
    info!("  Token: {}", args.token_address);
    info!("  Pool: {}", args.pool_address);
    info!("  Block: {}", block_number);
    info!("  ETH Levels - Current: {}, Simulated: {}, Threshold: {}", 
        current_eth_level, simulated_eth_level, eth_threshold);
    
    if args.skip_fk_validation {
        info!("Foreign key validation will be skipped for testing");
        
        // Temporarily disable foreign key constraints for this session
        match db_logger.execute_query("SET session_replication_role = 'replica';", &[]).await {
            Ok(_) => info!("Foreign key validation disabled"),
            Err(e) => {
                error!("Failed to disable foreign key validation: {}", e);
                return Err(eyre::eyre!("Database error: {}", e));
            }
        }
    }
    
    match db_logger.write_mempool_scam_prediction(
        &args.token_address,
        &args.pool_address,
        block_number,
        current_eth_level,
        simulated_eth_level,
        eth_threshold,
    ).await {
        Ok(_) => {
            info!("Successfully wrote prediction to database");
        },
        Err(e) => {
            error!("Failed to write prediction to database: {}", e);
            return Err(eyre::eyre!("Database write error: {}", e));
        }
    }
    
    if args.skip_fk_validation {
        // Restore foreign key constraints
        match db_logger.execute_query("SET session_replication_role = 'origin';", &[]).await {
            Ok(_) => info!("Foreign key validation restored"),
            Err(e) => {
                error!("Failed to restore foreign key validation: {}", e);
                // Continue anyway since we're finishing up
            }
        }
    }
    
    // Clean up test data if requested
    if !args.keep_test_data {
        info!("Cleaning up test data from database...");
        match db_logger.delete_test_records(
            &args.token_address,
            &args.pool_address,
            block_number,
        ).await {
            Ok(count) => {
                info!("Successfully removed {} test records from database", count);
            },
            Err(e) => {
                error!("Failed to clean up test data: {}", e);
                // Continue anyway since this is just cleanup
            }
        }
    } else {
        info!("Test data will be kept in the database (--keep-test-data flag used)");
    }
    
    info!("Database logger test completed successfully");
    Ok(())
} 