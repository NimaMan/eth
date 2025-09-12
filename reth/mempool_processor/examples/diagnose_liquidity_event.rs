/// Diagnose LP-approval + liquidity removal pair by inspecting processed tx data
///
/// - Loads processed transactions from local Reth DB using tx_processor::ProcessedTxProvider
/// - Prints approvals, input selector, and address balance changes (focus on ETH net deltas)
/// - Helps compare what FunctionDetector logged vs what the SignalDetector saw in state changes
///
/// Usage:
///   cargo run -p mempool_processor --example diagnose_liquidity_event \
///     -- <LP_APPROVAL_TX_HASH> <REMOVAL_TX_HASH> [RETH_DB_PATH]
///
/// Defaults (from your logs):
///   LP approval: 0x102a4037b9e3e09c6215c3440c8f715dd6e73e23a1cdb887cfed84328c863921
///   Removal:     0xaeb040ae1f729900071e75007efee190186ade2fd7072ecc5cfc56678194c545
///   DB path:     /home/nima/.local/share/reth/mainnet

use alloy_primitives::B256;
use eyre::Result;
use std::env;
use std::str::FromStr;
use tx_processor::ProcessedTxProvider;
use alloy_primitives::{Address, I256, U256};
use reth_chain_query::to_checksum_address;

fn parse_arg(idx: usize, default: &str) -> String {
    env::args().nth(idx).unwrap_or_else(|| default.to_string())
}

fn fourbyte_selector(input: &[u8]) -> Option<String> {
    if input.len() < 4 { return None; }
    Some(format!("{:02x}{:02x}{:02x}{:02x}", input[0], input[1], input[2], input[3]))
}

fn print_eth_balance_changes(tx: &tx_processor::ProcessedTransaction) {
    use std::collections::BTreeMap;
    let mut changes: BTreeMap<Address, I256> = BTreeMap::new();
    for (addr, delta) in &tx.address_balance_changes {
        if let Some(eth_u256) = delta.currency_net.get("ETH") {
            // Interpret U256 as signed via two's complement
            let eth_signed = I256::try_from(*eth_u256).unwrap_or(I256::ZERO);
            if eth_signed != I256::ZERO {
                changes.insert(*addr, eth_signed);
            }
        }
    }

    if changes.is_empty() {
        println!("  No ETH currency_net changes found.");
        return;
    }

    let mut entries: Vec<_> = changes.into_iter().collect();
    // Sort by absolute wei magnitude descending
    entries.sort_by(|a, b| {
        let av = a.1.unsigned_abs();
        let bv = b.1.unsigned_abs();
        bv.cmp(&av)
    });

    println!("  Top ETH balance deltas:");
    for (addr, v) in entries.iter().take(20) {
        let addr_str = to_checksum_address(addr);
        // Convert to ETH for readability
        let wei_str = v.to_string();
        let wei_i128 = wei_str.parse::<i128>().unwrap_or(0);
        let eth_f64 = (wei_i128 as f64) / 1e18;
        println!("    {}: {:+} wei ({:+.6} ETH)", addr_str, wei_i128, eth_f64);
    }
}

fn print_weth_token_deltas(tx: &tx_processor::ProcessedTransaction) {
    // Mainnet WETH address (checksum)
    let weth_addr = Address::from_slice(&hex::decode("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap());
    let weth_checksum = to_checksum_address(&weth_addr);
    let mut entries: Vec<(Address, I256)> = Vec::new();
    for (addr, delta) in &tx.address_balance_changes {
        if let Some(amount_u256) = delta.token_net.get(&weth_checksum) {
            let signed = I256::try_from(*amount_u256).unwrap_or(I256::ZERO);
            if signed != I256::ZERO {
                entries.push((*addr, signed));
            }
        }
    }
    if entries.is_empty() {
        println!("  No WETH token_net changes found.");
        return;
    }
    entries.sort_by(|a, b| b.1.unsigned_abs().cmp(&a.1.unsigned_abs()));
    println!("  Top WETH token_net deltas (wei):");
    for (addr, v) in entries.iter().take(20) {
        let addr_str = to_checksum_address(addr);
        let wei_str = v.to_string();
        let wei_i128 = wei_str.parse::<i128>().unwrap_or(0);
        let eth_f64 = (wei_i128 as f64) / 1e18;
        println!("    {}: {:+} ({:+.6} WETH)", addr_str, wei_i128, eth_f64);
    }
}

fn print_all_token_net(tx: &tx_processor::ProcessedTransaction) {
    use std::collections::BTreeMap;
    // Collect per-address token_net summaries
    let mut by_addr: BTreeMap<Address, Vec<(String, I256)>> = BTreeMap::new();
    for (addr, delta) in &tx.address_balance_changes {
        if delta.token_net.is_empty() { continue; }
        let mut entries: Vec<(String, I256)> = Vec::new();
        for (token_addr, amount_u256) in &delta.token_net {
            let signed = I256::try_from(*amount_u256).unwrap_or(I256::ZERO);
            if signed != I256::ZERO {
                entries.push((token_addr.clone(), signed));
            }
        }
        if !entries.is_empty() {
            // Sort by abs value desc
            entries.sort_by(|a, b| b.1.unsigned_abs().cmp(&a.1.unsigned_abs()));
            by_addr.insert(*addr, entries);
        }
    }
    if by_addr.is_empty() {
        println!("  No token_net changes found.");
        return;
    }
    println!("  token_net by address (top entries):");
    for (addr, entries) in by_addr {
        let addr_str = to_checksum_address(&addr);
        println!("    {}:", addr_str);
        for (token, amt) in entries.iter().take(10) {
            let amt_str = amt.to_string();
            // Also show in scientific style roughly via f64 if fits
            let approx = amt_str.parse::<f64>().unwrap_or(0.0) / 1e18;
            println!("      token {}: {} (~{:.6} in 18dp)", token, amt_str, approx);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let lp_hash = parse_arg(1, "0x102a4037b9e3e09c6215c3440c8f715dd6e73e23a1cdb887cfed84328c863921");
    let rem_hash = parse_arg(2, "0xaeb040ae1f729900071e75007efee190186ade2fd7072ecc5cfc56678194c545");
    let db_path = parse_arg(3, "/home/nima/.local/share/reth/mainnet");

    println!("🔎 Diagnose LP-Approval + Liquidity Removal");
    println!("DB: {}\n", db_path);

    let provider = ProcessedTxProvider::new(&db_path)?;

    // 1) LP approval
    let lp_b256 = B256::from_str(&lp_hash)?;
    println!("== LP Approval ==\nHash: {}", lp_hash);
    match provider.process_transaction_by_hash(lp_b256).await {
        Ok(tx) => {
            println!("  Block: {} | From: {} | To: {}", tx.block_number, tx.from_address, tx.to_address.unwrap_or_default());
            if let Some(sel) = fourbyte_selector(&tx.input) {
                println!("  4-byte selector: {}", sel);
            }
            if !tx.approvals.is_empty() {
                println!("  Approvals ({}):", tx.approvals.len());
                for (i, a) in tx.approvals.iter().enumerate() {
                    println!("    #{} owner={} spender={} amount={}", i + 1, a.owner, a.spender, a.amount);
                }
            } else {
                println!("  No approvals decoded.");
            }
            if !tx.erc20_transfers.is_empty() {
                println!("  ERC20 Transfers: {}", tx.erc20_transfers.len());
            }
            println!("  Address balance changes (ETH):");
            print_eth_balance_changes(&tx);
        }
        Err(e) => {
            println!("  ERROR fetching processed tx: {}", e);
        }
    }

    // 2) Liquidity removal
    let rem_b256 = B256::from_str(&rem_hash)?;
    println!("\n== Liquidity Removal ==\nHash: {}", rem_hash);
    match provider.process_transaction_by_hash(rem_b256).await {
        Ok(tx) => {
            println!("  Block: {} | From: {} | To: {}", tx.block_number, tx.from_address, tx.to_address.unwrap_or_default());
            if let Some(sel) = fourbyte_selector(&tx.input) {
                println!("  4-byte selector: {}", sel);
            }
            println!("  Address balance changes (ETH):");
            print_eth_balance_changes(&tx);
            println!("  Address balance changes (token_net):");
            print_all_token_net(&tx);
            println!("  Address balance changes (WETH token_net):");
            print_weth_token_deltas(&tx);
            if !tx.uniswap_v2_swaps.is_empty() {
                println!("  UniswapV2 Swaps: {}", tx.uniswap_v2_swaps.len());
            }
            if !tx.internal_transactions.is_empty() {
                println!("  Internal txs: {}", tx.internal_transactions.len());
            }
        }
        Err(e) => {
            println!("  ERROR fetching processed tx: {}", e);
        }
    }

    println!("\nTip: Look for a large negative ETH delta on a pool address in the removal tx.\n");
    Ok(())
}
