use eyre::Result;
use reth_chain_query::reth_index::RethIndexDB;

fn main() -> Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/nima/.local/share/reth/mainnet/reth_index".to_string());
    let db = RethIndexDB::open(&dir)?;
    let count = db.count_tx_arrivals()?;
    println!("tx_arrival entries: {}", count);
    Ok(())
}
