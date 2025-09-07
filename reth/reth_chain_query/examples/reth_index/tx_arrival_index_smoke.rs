use eyre::Result;
use reth_chain_query::reth_index::RethIndexDB;

fn now_ns() -> u64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    now.as_secs() * 1_000_000_000 + now.subsec_nanos() as u64
}

fn main() -> Result<()> {
    // Example usage: writes a dummy tx_number and reads it back
    let dir = std::env::args().nth(1).unwrap_or_else(|| "./.reth_index_tmp".to_string());
    let db = RethIndexDB::open(&dir)?;
    let tx_number: u64 = 123_456_789;
    let first_seen = now_ns();
    // store in milliseconds
    let first_seen_ms = first_seen / 1_000_000;
    db.put_tx_arrival_ms(tx_number, first_seen_ms)?;
    let read_back = db.get_tx_arrival_ms(tx_number)?;
    println!("Wrote arrival: tx_number={} first_seen_ns={}", tx_number, first_seen);
    println!("Read back: {:?}", read_back);
    Ok(())
}
