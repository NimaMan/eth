use eyre::Result;
use reth_chain_query::reth_index::RethIndexDB;

fn main() -> Result<()> {
    let dir = match std::env::args().nth(1) {
        Some(dir) => dir,
        None => format!("{}/reth_index", tx_simulator::config::repo::reth_datadir()?),
    };
    let db = RethIndexDB::open(&dir)?;
    let count = db.count_tx_arrivals()?;
    println!("tx_arrival entries: {}", count);
    Ok(())
}
