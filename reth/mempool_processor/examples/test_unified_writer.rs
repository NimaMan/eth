use eyre::Result;
use mempool_processor::db_writers::UnifiedSignalWriter;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,mempool_processor=debug")
        .init();

    println!("Testing unified signal writer initialization...\n");

    let database_url = "postgresql://postgres:postgres@localhost:5432/eth_db";

    match UnifiedSignalWriter::new(database_url).await {
        Ok(writer) => {
            println!("✅ Unified signal writer initialized successfully!");
            println!("{}", writer.get_status());
            println!("\nWriter has active connections: {}", writer.has_writers());
        }
        Err(e) => {
            println!("❌ Failed to initialize unified signal writer: {}", e);
        }
    }

    Ok(())
}
