use eyre::Result;
use std::env;
use reth_chain_query::reth_index::RethIndexDB;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let dir = args.next().expect("usage: delete_tx_arrival <index_dir> <tx_number>");
    let tx_number: u64 = args
        .next()
        .expect("missing tx_number")
        .parse()
        .expect("invalid tx_number");
    let db = RethIndexDB::open(&dir)?;
    let existed = db.delete_tx_arrival(tx_number)?;
    println!("delete tx_number={} existed={} in {}", tx_number, existed, dir);
    Ok(())
}

