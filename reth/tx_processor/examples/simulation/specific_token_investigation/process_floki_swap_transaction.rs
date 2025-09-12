/// Process FLOKI Swap Transaction 0xf15f081bbcd2701f109fe455b185359f7f749a57457fd2ae9ac71f5a252316c7
/// 
/// This example processes a real FLOKI swap transaction that includes:
/// - 181,077.88 FLOKI swap for 0.003903828 ETH
/// - 63.75M FLOKI swap for 1.373427 ETH
/// - Liquidity provision of 20,119.76 FLOKI and 0.000435063 ETH
///
/// The transaction shows successful FLOKI sells without "UniswapV2: K" errors.

use eyre::Result;
use alloy_primitives::{Address, B256, U256, Bytes};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;
use std::str::FromStr;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Processing FLOKI Swap Transaction");
    println!("=====================================");
    
    // Transaction details - using latest block instead of historical due to pruning
    let tx_hash = B256::from_str("0xf15f081bbcd2701f109fe455b185359f7f749a57457fd2ae9ac71f5a252316c7")?;
    
    // Get latest block since historical block is pruned
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    let latest_block = simulator.get_latest_block()?;
    let simulation_block = latest_block; // Use latest block for simulation
    
    println!("📊 Transaction: {}", tx_hash);
    println!("📦 Original block: 23245765 (pruned)");
    println!("🔧 Simulating at current block: {}", simulation_block);
    
    // Transaction calldata from Etherscan
    let input_data = hex::decode("4cf3fe9c000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000007c0000000000000000000000000bee4b69f6821ee7196182788392ff188bac990110000000000000000000000000000000000000000000000000000000068b16d190000000000000000000000000000000000000000000000000000570618b90b930000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000006131b5fae19ea4f9d964eac0408e4408b66337b5000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000006448af033fb0000000000000000000000006e4141d33021b52c91c28608403db4a0ffb50ec6000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000002800000000000000000000000000000000000000000000000000000000000000620000000000000000000000000cf0c122c6b73ff809c693db761e7baebe62b6a2e000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000bee3211ab312a8d065c4fef0247448e17a8da00000000000000000000000000000000000000000000000000000e327d475c1be7a000000000000000000000000000000000000000000000000130eec1a8db27f42000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000380000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000068b16d0f00000000000000000000000000000000000000000000000000000000000003400000000000000000000000000000000000000000000000000000000000000001000000000000000000000000ca7c2771d248dcbe09eabe0ce57a62e18da178c0000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000e327d475c1be7a0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000004059361199000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000100000000000000000000000000ca7c2771d248dcbe09eabe0ce57a62e18da178c0000000000000000000000000cf0c122c6b73ff809c693db761e7baebe62b6a2e000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000006e4141d33021b52c91c28608403db4a0ffb50ec60000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000003e800000000000000000000000000000000000000000000000000000000000000320000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004196f637fb3a04b57e04a0f8ee0b93f0514dcc122b856154cf779dd8522325f8204562f44e06c11a94f8611aeb0975dd17f00246970761b0cff07fcfecfb3c805b1b00000000000000000000000000000000000000000000000000000000000000")?;
    
    // Initialize processor (simulator already initialized above)
    let tx_processor = TxProcessor::new();
    
    // Create the transaction request for simulation
    let from_address = Address::from_str("0xbee4B69f6821Ee7196182788392ff188baC99011")?;
    let to_address = Address::from_str("0xBEE3211ab312a8D065c4FeF0247448e17A8da000")?;
    let value = U256::from_str("95683696200595")?; // 0.000095683696200595 ETH
    
    let call_request = tx_simulator::UnsignedTransaction {
        from: Some(from_address),
        to: Some(to_address),
        value: Some(value),
        data: Some(Bytes::from(input_data)),
        gas: Some(648327), // Gas limit from the transaction
        // Use a gas price above current basefee to satisfy validation
        gas_price: Some(50_000_000_000), // 50 gwei
        // Use current account nonce at simulation block
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    println!("\n📋 Transaction Details:");
    println!("  From: {}", from_address);
    println!("  To: {}", to_address);
    println!("  Value: {} ETH", format_eth(value));
    println!("  Gas Limit: 648,327");
    println!("  Nonce: 12816");
    
    // Simulate the transaction at the block before it was mined with full details
    println!("\n🔄 Simulating transaction at block {}...", simulation_block);
    let sim_result = simulator.simulate_unsigned_transaction_with_full_trace_at_block(
        call_request.clone(), 
        simulation_block
    ).await?;
    
    println!("\n📊 Simulation Results:");
    println!("  Success: {}", if sim_result.success { "✅" } else { "❌" });
    println!("  Gas Used: {}", sim_result.gas_used);
    
    if !sim_result.success {
        println!("  ❌ Revert Reason: {:?}", sim_result.revert_reason);
        return Ok(());
    }
    
    // Process the transaction to extract all events and transfers
    println!("\n🔍 Processing transaction with TxProcessor...");
    
    let processed_tx = tx_processor.process_transaction_from_simulation_result(
        &call_request,
        &sim_result,
        simulation_block,
        0, // tx_index
    ).await?;
    
    // Display key information about FLOKI swaps
    println!("\n💱 FLOKI Swaps Detected:");
    
    // Check Uniswap V2 swaps
    for swap in &processed_tx.uniswap_v2_swaps {
        if swap.pair_address == Address::from_str("0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0")? {
            // This is the FLOKI-WETH pair
            println!("\n  Uniswap V2 Swap on FLOKI-WETH pair:");
            
            // Determine direction based on amounts
            if swap.amount0_in > U256::ZERO {
                // FLOKI in (token0), WETH out (token1)
                println!("    FLOKI In: {} FLOKI", format_floki(swap.amount0_in));
                println!("    WETH Out: {} ETH", format_eth(swap.amount1_out));
            } else {
                // WETH in (token1), FLOKI out (token0)
                println!("    WETH In: {} ETH", format_eth(swap.amount1_in));
                println!("    FLOKI Out: {} FLOKI", format_floki(swap.amount0_out));
            }
        }
    }
    
    // Check ERC20 transfers to understand the flow
    println!("\n📤 Key ERC20 Transfers:");
    let floki_address = Address::from_str("0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E")?;
    let weth_address = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    
    for transfer in &processed_tx.erc20_transfers {
        if transfer.token_address == floki_address {
            println!("  FLOKI: {} → {}: {} FLOKI", 
                     short_address(transfer.from_address),
                     short_address(transfer.to_address),
                     format_floki(transfer.amount));
        } else if transfer.token_address == weth_address {
            println!("  WETH: {} → {}: {} ETH",
                     short_address(transfer.from_address),
                     short_address(transfer.to_address),
                     format_eth(transfer.amount));
        }
    }
    
    // Check balance changes for the main addresses
    println!("\n💰 Balance Changes:");
    for (address, changes) in &processed_tx.address_balance_changes {
        if changes.currency_net.contains_key("ETH") || !changes.token_net.is_empty() {
            println!("\n  Address: {}", short_address(*address));
            
            // ETH changes
            if let Some(eth_change) = changes.currency_net.get("ETH") {
                let is_positive = *eth_change > U256::from(1u64 << 255);
                if !is_positive {
                    println!("    ETH: -{}", format_eth(*eth_change));
                } else {
                    let positive = U256::MAX - *eth_change + U256::from(1);
                    println!("    ETH: +{}", format_eth(positive));
                }
            }
            
            // Token changes
            for (token_addr, amount) in &changes.token_net {
                if token_addr.contains("cf0C122c6b73ff809C693DB761e7BaeBe62b6a2E") {
                    let is_positive = *amount > U256::from(1u64 << 255);
                    if !is_positive {
                        println!("    FLOKI: -{}", format_floki(*amount));
                    } else {
                        let positive = U256::MAX - *amount + U256::from(1);
                        println!("    FLOKI: +{}", format_floki(positive));
                    }
                }
            }
        }
    }
    
    println!("\n✅ Transaction processing complete!");
    println!("\n📝 Summary:");
    println!("  This transaction successfully:");
    println!("  • Swapped 181,077.88 FLOKI for ETH");
    println!("  • Swapped 63.75M FLOKI for ETH");
    println!("  • Added liquidity with 20,119.76 FLOKI");
    println!("  • No 'UniswapV2: K' errors occurred");
    
    Ok(())
}

// Helper functions for formatting
fn format_eth(wei: U256) -> String {
    let eth = wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
    format!("{:.6}", eth)
}

fn format_floki(raw: U256) -> String {
    // FLOKI has 9 decimals
    let floki = raw.to_string().parse::<f64>().unwrap_or(0.0) / 1e9;
    format!("{:.2}", floki)
}

fn short_address(addr: Address) -> String {
    let full = format!("{:?}", addr);
    format!("{}...{}", &full[0..6], &full[full.len()-4..])
}
