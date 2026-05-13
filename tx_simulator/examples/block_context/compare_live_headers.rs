use eyre::{eyre, Result, WrapErr};
use reqwest::Client;
use reth_primitives_traits::SealedHeader;
use serde_json::{json, Value};
use tx_simulator::block_context::header_json::parse_sealed_header_from_json;
use tx_simulator::block_context::{live_chain_cache::LiveChainCache, resolve_live_data_redis_url};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";

#[tokio::main]
async fn main() -> Result<()> {
    let redis_url = resolve_live_data_redis_url();
    let cache =
        LiveChainCache::new(&redis_url).wrap_err("failed to connect to live chain cache")?;

    let mut blocks = cache.recent_block_numbers(5).await?;
    if blocks.is_empty() {
        return Err(eyre!("no blocks available in Redis live cache"));
    }
    blocks.sort();
    println!("Comparing headers for blocks: {:?}", blocks);

    for block_number in blocks {
        let redis_payload = cache
            .fetch_block_header(block_number)
            .await?
            .ok_or_else(|| eyre!("missing Redis header for block {}", block_number))?;
        let redis_header = parse_sealed_header_from_json(&redis_payload)
            .wrap_err_with(|| format!("failed to parse Redis header for block {block_number}"))?;

        let chain_header = fetch_chain_header(block_number).await?;
        report_diff(block_number, &redis_header, &chain_header);
    }

    Ok(())
}

async fn fetch_chain_header(block_number: u64) -> Result<SealedHeader> {
    let rpc_url = std::env::var("ETH_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getBlockByNumber",
        "params": [format!("0x{:x}", block_number), false],
    });

    let client = Client::new();
    let response: Value = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .wrap_err("failed to send RPC request")?
        .json()
        .await
        .wrap_err("failed to decode RPC response")?;

    let result = response
        .get("result")
        .cloned()
        .ok_or_else(|| eyre!("RPC response missing result field"))?;
    if result.is_null() {
        return Err(eyre!(
            "RPC returned null result (potentially unknown block)"
        ));
    }
    let header_json = serde_json::to_string(&result)?;
    parse_sealed_header_from_json(&header_json)
}

fn report_diff(block_number: u64, redis: &SealedHeader, chain: &SealedHeader) {
    let r = redis.header();
    let c = chain.header();
    let mut mismatches = 0usize;

    macro_rules! cmp_field {
        ($label:expr, $left:expr, $right:expr) => {
            if $left != $right {
                mismatches += 1;
                println!(
                    "  - {} mismatch\n      redis: {:?}\n      chain: {:?}",
                    $label, $left, $right
                );
            }
        };
    }

    println!("Block {block_number}: comparing header fields");
    cmp_field!("hash", redis.hash(), chain.hash());
    cmp_field!("parent_hash", r.parent_hash, c.parent_hash);
    cmp_field!("state_root", r.state_root, c.state_root);
    cmp_field!(
        "transactions_root",
        r.transactions_root,
        c.transactions_root
    );
    cmp_field!("receipts_root", r.receipts_root, c.receipts_root);
    cmp_field!("logs_bloom", r.logs_bloom, c.logs_bloom);
    cmp_field!("difficulty", r.difficulty, c.difficulty);
    cmp_field!("number", r.number, c.number);
    cmp_field!("gas_limit", r.gas_limit, c.gas_limit);
    cmp_field!("gas_used", r.gas_used, c.gas_used);
    cmp_field!("timestamp", r.timestamp, c.timestamp);
    cmp_field!("extra_data", r.extra_data, c.extra_data);
    cmp_field!("mix_hash", r.mix_hash, c.mix_hash);
    cmp_field!("nonce", r.nonce, c.nonce);
    cmp_field!("base_fee_per_gas", r.base_fee_per_gas, c.base_fee_per_gas);
    cmp_field!("blob_gas_used", r.blob_gas_used, c.blob_gas_used);
    cmp_field!("excess_blob_gas", r.excess_blob_gas, c.excess_blob_gas);
    cmp_field!(
        "parent_beacon_block_root",
        r.parent_beacon_block_root,
        c.parent_beacon_block_root
    );
    cmp_field!("withdrawals_root", r.withdrawals_root, c.withdrawals_root);

    if mismatches == 0 {
        println!("Block {block_number}: headers match");
    } else {
        println!("Block {block_number}: {mismatches} header field mismatch(es)");
    }
}
