/// Process Real Transaction Example
/// 
/// This example shows how to process a real transaction and extract all decoded information
/// just like Python's eth_block_processor.txn module

use tx_processor::{tx_processor::TxProcessor, ProcessedTransaction};
use alloy_primitives::{Address, B256, U256, Bytes, Log as AlloyLog, LogData};
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Initialize the processor
    let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;
    tracing::info!("✅ TX Processor initialized");
    
    // Example: Process a USDC transfer transaction
    // This would typically come from RPC or database
    let tx_hash = B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")?;
    let block_number = 20_000_000;
    let block_timestamp = 1700000000;
    let tx_index = 42;
    let from = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f8fA49")?;
    let to = Some(Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?); // USDC
    let value = U256::ZERO;
    let gas_price = U256::from(20_000_000_000u64); // 20 gwei
    let gas_used = 65_000;
    let status = "success".to_string();
    let nonce = 100;
    
    // ERC20 transfer calldata: transfer(address,uint256)
    let input = hex::decode("a9059cbb000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7000000000000000000000000000000000000000000000000000000174876e800")?;
    
    // Create sample logs (normally from receipt)
    let logs = vec![
        // ERC20 Transfer event
        AlloyLog::new_unchecked(
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?, // USDC
            vec![
                B256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")?, // Transfer topic
                B256::from_str("0x000000000000000000000000742d35cc6634c0532925a3b844bc9e7595f8fa49")?, // from
                B256::from_str("0x000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7")?, // to
            ],
            Bytes::from(hex::decode("000000000000000000000000000000000000000000000000000000174876e800")?), // amount
        ),
    ];
    
    // Process the transaction
    let processed_tx = processor.process_transaction(
        tx_hash,
        block_number,
        block_timestamp,
        tx_index,
        from,
        to,
        value,
        input,
        gas_price,
        gas_used,
        status,
        nonce,
        logs,
    ).await?;
    
    // Print results
    print_processed_transaction(&processed_tx);
    
    Ok(())
}

fn print_processed_transaction(tx: &ProcessedTransaction) {
    println!("\n📊 Processed Transaction Summary");
    println!("================================");
    println!("Hash: {}", tx.hash);
    println!("Block: {} (timestamp: {})", tx.block_number, tx.block_timestamp);
    println!("From: {}", tx.from_address);
    println!("To: {:?}", tx.to_address);
    println!("Value: {} ETH", format_ether(tx.value));
    println!("Status: {}", tx.status);
    println!("Type: {}", tx.txn_type);
    
    println!("\n💰 Fees:");
    println!("  Gas Price: {} gwei", format_gwei(tx.fees.gas_price));
    println!("  Gas Used: {}", tx.fees.gas_used);
    println!("  Total Fee: {} ETH", format_ether(tx.fees.txn_fee));
    
    if !tx.actions.is_empty() {
        println!("\n🎯 Actions Identified:");
        for action in &tx.actions {
            println!("  - {}", action);
        }
    }
    
    if !tx.erc20_transfers.is_empty() {
        println!("\n💸 ERC20 Transfers:");
        for transfer in &tx.erc20_transfers {
            println!("  Token: {}", transfer.token_address);
            println!("  From: {}", transfer.from_address);
            println!("  To: {}", transfer.to_address);
            println!("  Amount: {}", transfer.amount);
        }
    }
    
    if !tx.approvals.is_empty() {
        println!("\n✅ Approvals:");
        for approval in &tx.approvals {
            println!("  Token: {}", approval.token_address);
            println!("  Owner: {}", approval.owner);
            println!("  Spender: {}", approval.spender);
            println!("  Amount: {}", approval.amount);
        }
    }
    
    if !tx.uniswap_v2_swaps.is_empty() || !tx.uniswap_v3_swaps.is_empty() {
        println!("\n🔄 DEX Swaps:");
        for swap in &tx.uniswap_v2_swaps {
            println!("  V2 Swap on {}", swap.pair_address);
            println!("    In: {} token0, {} token1", swap.amount0_in, swap.amount1_in);
            println!("    Out: {} token0, {} token1", swap.amount0_out, swap.amount1_out);
        }
        for swap in &tx.uniswap_v3_swaps {
            println!("  V3 Swap on {}", swap.pool_address);
            println!("    Amount0: {}", swap.amount0);
            println!("    Amount1: {}", swap.amount1);
            println!("    Price: {}", swap.sqrt_price_x96);
        }
    }
    
    if !tx.state_changes.is_empty() {
        println!("\n📝 State Changes:");
        for (addr, changes) in &tx.state_changes {
            println!("  Address {}: {}", addr, serde_json::to_string_pretty(changes).unwrap());
        }
    }
    
    println!("\n📍 Unique Addresses Involved: {}", tx.unique_addresses.len());
    println!("🪙 ERC20 Tokens Involved: {}", tx.erc20_contracts.len());
}

fn format_ether(value: U256) -> String {
    let eth = value.to_string();
    if eth.len() > 18 {
        let (whole, decimal) = eth.split_at(eth.len() - 18);
        format!("{}.{}", whole, &decimal[..6])
    } else {
        format!("0.{:0>18}", eth)
    }
}

fn format_gwei(value: U256) -> String {
    (value / U256::from(1_000_000_000u64)).to_string()
}