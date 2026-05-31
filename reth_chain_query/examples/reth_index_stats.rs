use std::env;
use std::path::PathBuf;

use eyre::Result;
use reth_chain_query::reth_index::{
    tables::MempoolTxArrivalTable, AddressBlockParticipationIndex, RethIndexDB,
};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/nima/storage/samsung8tb/ethereum/reth/reth_index"));

    let db = RethIndexDB::open_read_only(&path)?;
    let env = db.env();
    let stat = env.stat()?;
    let info = env.info()?;
    let freelist = env.freelist()?;
    let page_size = stat.page_size() as usize;
    let last_pages = info.last_pgno() + 1;
    let used_pages = last_pages.saturating_sub(freelist);
    let map_pages = info.map_size() / page_size;
    let unused_map_tail_pages = map_pages.saturating_sub(last_pages);
    let free_pages = freelist.saturating_add(unused_map_tail_pages);

    println!("path={}", path.display());
    println!("page_size={page_size}");
    println!("map_size_bytes={}", info.map_size());
    println!("last_pgno={}", info.last_pgno());
    println!("last_pages={last_pages}");
    println!("freelist_pages={freelist}");
    println!("used_pages={used_pages}");
    println!("mapped_pages={map_pages}");
    println!("unused_map_tail_pages={unused_map_tail_pages}");
    println!("free_pages={free_pages}");
    println!("used_bytes={}", used_pages.saturating_mul(page_size));
    println!("freelist_bytes={}", freelist.saturating_mul(page_size));
    println!(
        "unused_map_tail_bytes={}",
        unused_map_tail_pages.saturating_mul(page_size)
    );
    println!("free_bytes={}", free_pages.saturating_mul(page_size));
    println!("last_txnid={}", info.last_txnid());
    println!("oldest_reader_txnid={}", info.latter_reader_txnid());
    println!("max_readers={}", info.max_readers());
    println!("num_readers={}", info.num_readers());
    println!();

    let tx = env.begin_ro_txn()?;
    for table in [
        AddressBlockParticipationIndex::TABLE_NAME,
        MempoolTxArrivalTable::TABLE_NAME,
    ] {
        let table_db = tx.open_db(Some(table))?;
        let table_stat = tx.db_stat(table_db.dbi())?;
        let pages = table_stat
            .branch_pages()
            .saturating_add(table_stat.leaf_pages())
            .saturating_add(table_stat.overflow_pages());
        println!("table={table}");
        println!("  entries={}", table_stat.entries());
        println!("  depth={}", table_stat.depth());
        println!("  branch_pages={}", table_stat.branch_pages());
        println!("  leaf_pages={}", table_stat.leaf_pages());
        println!("  overflow_pages={}", table_stat.overflow_pages());
        println!("  total_pages={pages}");
        println!("  approx_bytes={}", pages.saturating_mul(page_size));
    }

    Ok(())
}
