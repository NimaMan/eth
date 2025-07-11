//! Database connection example - demonstrates connecting to PostgreSQL

use qarqa_core_types::*;
use qarqa_data_access::{DatabaseManager};
use tracing::{info, warn, error};
use sqlx::Row;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Database Connection Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting database connection example");
    
    // Try to connect to database with different URLs
    let database_urls = vec![
        // Primary database URL (from environment or default)
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string()),
        
        // Alternative local database
        "postgresql://postgres:password@localhost/eth_db".to_string(),
        
        // Invalid URL for testing error handling
        "postgresql://invalid:invalid@nonexistent/baddb".to_string(),
    ];
    
    for (i, url) in database_urls.iter().enumerate() {
        println!("{}. Testing connection to database:", i + 1);
        println!("   URL: {}", mask_password(url));
        
        match DatabaseManager::new(url).await {
            Ok(db) => {
                println!("   ✓ Connection successful!");
                
                // Test health check
                match db.health_check().await {
                    Ok(()) => {
                        println!("   ✓ Health check passed");
                        
                        // Show connection pool info
                        let pool_info = get_pool_info(&db);
                        println!("   Pool info: {}", pool_info);
                        
                        // This is our working database, let's use it for more tests
                        if i == 0 || i == 1 {
                            test_database_operations(&db).await?;
                        }
                        
                        break; // Stop trying other URLs since this one works
                    }
                    Err(e) => {
                        warn!("Health check failed: {}", e);
                        println!("   ⚠ Health check failed: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Connection failed: {}", e);
                println!("   ✗ Connection failed: {}", e);
                
                if i < database_urls.len() - 1 {
                    println!("   → Trying next database URL...");
                } else {
                    println!("   → All database connection attempts failed");
                    println!("   → This is expected if no PostgreSQL database is running");
                }
            }
        }
        println!();
    }
    
    // Demonstrate connection pooling concepts
    demonstrate_connection_pooling().await;
    
    println!("=== Database connection example completed! ===");
    
    Ok(())
}

async fn test_database_operations(db: &DatabaseManager) -> QarqaResult<()> {
    println!("   Testing basic database operations:");
    
    // Test simple query (should work on any PostgreSQL database)
    match sqlx::query("SELECT 1 as test_value")
        .fetch_one(db.pool())
        .await 
    {
        Ok(row) => {
            let test_value: i32 = row.get("test_value");
            println!("     ✓ Simple query successful: {}", test_value);
        }
        Err(e) => {
            println!("     ✗ Simple query failed: {}", e);
        }
    }
    
    // Test checking if our tables exist (won't fail if they don't)
    let tables_to_check = vec!["transactions", "participants", "blocks"];
    
    for table in tables_to_check {
        match sqlx::query("SELECT COUNT(*) as count FROM information_schema.tables WHERE table_name = $1")
            .bind(table)
            .fetch_one(db.pool())
            .await
        {
            Ok(row) => {
                let count: i64 = row.get("count");
                if count > 0 {
                    println!("     ✓ Table '{}' exists", table);
                } else {
                    println!("     ⚠ Table '{}' does not exist (expected for new database)", table);
                }
            }
            Err(e) => {
                println!("     ✗ Error checking table '{}': {}", table, e);
            }
        }
    }
    
    Ok(())
}

async fn demonstrate_connection_pooling() {
    println!("Connection Pooling Concepts:");
    println!("  • PostgreSQL connections are expensive to create");
    println!("  • Connection pools reuse existing connections");
    println!("  • Default pool size: 10 connections");
    println!("  • Connections are automatically returned to pool");
    println!("  • Pool handles connection health and timeouts");
    println!("  • Multiple components can share the same pool");
    println!();
}

fn mask_password(url: &str) -> String {
    // Simple password masking for display
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(slash_pos) = url[..colon_pos].rfind('/') {
                let before = &url[..slash_pos + 1];
                let user_start = slash_pos + 1;
                if let Some(user_colon) = url[user_start..colon_pos].find(':') {
                    let user = &url[user_start..user_start + user_colon];
                    let after = &url[at_pos..];
                    return format!("{}{}:***{}", before, user, after);
                }
            }
        }
    }
    url.to_string()
}

fn get_pool_info(_db: &DatabaseManager) -> String {
    // In a real implementation, you'd get actual pool statistics
    // For this example, we'll show conceptual information
    format!(
        "max_connections=10, active=1, idle=0, waiting=0"
    )
}