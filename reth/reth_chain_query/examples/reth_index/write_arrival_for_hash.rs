use alloy_primitives::B256;
use eyre::Result;
use reth_chain_query::reth_index::{
    database::RethIndexDB, writers::mempool_arrival_writer::MempoolArrivalWriter,
};
use std::sync::Arc;
use tx_simulator::TxSimulator;

fn parse_hash(s: &str) -> B256 {
    let h = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(h).expect("invalid hex");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    B256::from(arr)
}

fn now_ms() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    (now.as_secs() as u64) * 1000 + (now.subsec_millis() as u64)
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let reth_datadir = match args.next() {
        Some(path) => path,
        None => tx_simulator::config::repo::reth_datadir()?,
    };
    let index_dir = args
        .next()
        .unwrap_or_else(|| format!("{}/reth_index", reth_datadir));
    let hash_hex = args
        .next()
        .expect("usage: write_arrival_for_hash <reth_datadir> <index_dir> <tx_hash> [rpc_url]");
    let rpc_url = args.next();

    let sim = TxSimulator::new(&reth_datadir)?;
    let provider_factory = Arc::new(sim.provider_factory().clone());
    let db = Arc::new(RethIndexDB::open(&index_dir)?);
    let writer = Arc::new(match rpc_url {
        Some(rpc_url) => MempoolArrivalWriter::new_with_fallbacks(
            db.clone(),
            provider_factory,
            reth_datadir.clone(),
            rpc_url,
        ),
        None => MempoolArrivalWriter::new(db.clone(), provider_factory),
    });

    let hash = parse_hash(&hash_hex);
    let ts_ms = now_ms();
    let written = writer.write_arrivals_by_hashes_ms_return_resolved(&[(hash, ts_ms)])?;
    println!("resolved_and_written: {}", written.len());
    if let Some(tx) = written.first() {
        println!("first hash written: 0x{:x}", tx);
    }
    println!("count after: {}", db.count_tx_arrivals()?);
    Ok(())
}
