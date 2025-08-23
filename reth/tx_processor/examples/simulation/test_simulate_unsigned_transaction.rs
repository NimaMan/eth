/// Test simulate_unsigned_transaction function
/// 
/// This example tests the simulate_unsigned_transaction function using a real transaction
/// and compares the simulation results with expected address balance changes.

use tx_processor::{TxProcessor, CallRequest};
use alloy_primitives::{Address, U256, Bytes, B256};
use std::str::FromStr;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the TX Processor
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let processor = TxProcessor::new(reth_datadir)?;
    
    // Original transaction data from hash 0x343d16a8c63e7f8475b69340437a9389daf10bbcd944e7ff86adafdb05a606e8
    let tx_hash = B256::from_str("0x343d16a8c63e7f8475b69340437a9389daf10bbcd944e7ff86adafdb05a606e8")?;
    
    // Create unsigned transaction (CallRequest) from the transaction data
    let call_request = CallRequest {
        from: Some(Address::from_str("0x6D356ab697B0AB2A871B6Be79073A35664340440")?),
        to: Some(Address::from_str("0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e")?),
        value: Some(U256::ZERO), // 0.0 ETH
        data: Some(Bytes::from_str("0x3d0e3ec50000000000000000000000000000000000000000000000000001f5e2314ed46d00000000000000000000000000000000000000000000000000400a5ba681e86f00000000000000000000000000000000000000000000000000000000000000c00000000000000000000000006d356ab697b0ab2a871b6be79073a356643404400000000000000000000000000000000000000000000000000000000068790aec0000000000000000000000005c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000f2e24d564a08a3acc31985ebd3c6ee7fa9943a84000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2")?),
        gas: Some(600000), // Sufficient gas
        gas_price: Some(21171207287), // From original tx
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None, // Let simulator determine current nonce
    };
    
    println!("🔄 Simulating unsigned transaction...");
    println!("From: {:?}", call_request.from);
    println!("To: {:?}", call_request.to);
    println!("Value: {:?}", call_request.value);
    println!("Gas: {:?}", call_request.gas);
    println!("Nonce: {:?}", call_request.nonce);
    
    // Simulate the unsigned transaction at one block before the original transaction
    // Original transaction was at block 22939570, so simulate at 22939569
    let simulation_block = 22939569;
    let processed_tx = match processor.simulate_unsigned_transaction_with_logs_and_address_balance_changes(call_request.clone(), Some(simulation_block)).await {
        Ok(detailed_result) => {
            println!("✅ Detailed simulation at block {} successful!", simulation_block);
            println!("   Success: {}", detailed_result.success);
            println!("   Gas used: {}", detailed_result.gas_used);
            println!("   Logs count: {}", detailed_result.logs.len());
            println!("   Address balance changes count: {}", detailed_result.address_balance_changes.len());
            
            // For testing, create a minimal ProcessedTransaction with the simulation results
            let mut test_tx = tx_processor::ProcessedTransaction::new(
                B256::ZERO,
                simulation_block,
                0,
                0,
                call_request.from.unwrap_or(Address::ZERO),
                call_request.to,
                call_request.value.unwrap_or(U256::ZERO),
                if detailed_result.success { "1".to_string() } else { "0".to_string() },
                0,
                call_request.data.map(|d| d.to_vec()).unwrap_or_default(),
            );
            
            // Add address balance changes
            for (addr, changes) in detailed_result.address_balance_changes {
                test_tx.address_balance_changes.insert(
                    addr,
                    serde_json::json!({
                        "eth_net": changes.eth_net,
                        "token_net": changes.token_net,
                    })
                );
            }
            
            test_tx
        }
        Err(e) => {
            println!("❌ Detailed simulation failed: {}", e);
            println!("🔄 Trying current block simulation instead...");
            processor.process_unsigned_transaction(call_request).await?
        }
    };
    
    println!("\n✅ Simulation completed!");
    println!("Transaction type: {}", processed_tx.txn_type);
    println!("Status: {}", processed_tx.status);
    println!("Gas used: {}", processed_tx.fees.gas_used);
    
    // Print ERC20 transfers
    println!("\n📋 ERC20 Transfers ({}):", processed_tx.erc20_transfers.len());
    for (i, transfer) in processed_tx.erc20_transfers.iter().enumerate() {
        println!("  {}. Token: {:?}", i + 1, transfer.token_address);
        println!("     From: {:?}", transfer.from_address);
        println!("     To: {:?}", transfer.to_address);
        println!("     Amount: {}", transfer.amount);
    }
    
    // Print internal transactions
    println!("\n📋 Internal Transactions ({}):", processed_tx.internal_transactions.len());
    for (i, internal_tx) in processed_tx.internal_transactions.iter().enumerate() {
        println!("  {}. From: {:?}", i + 1, internal_tx.from_address);
        println!("     To: {:?}", internal_tx.to_address);
        println!("     Value: {}", internal_tx.value);
        println!("     Type: {}", internal_tx.trace_type);
        println!("     Depth: {}", internal_tx.depth);
    }
    
    // Print Uniswap swaps
    println!("\n📋 Uniswap V2 Swaps ({}):", processed_tx.uniswap_v2_swaps.len());
    for (i, swap) in processed_tx.uniswap_v2_swaps.iter().enumerate() {
        println!("  {}. Pair: {:?}", i + 1, swap.pair_address);
        println!("     Sender: {:?}", swap.sender);
        println!("     Amount0In: {}", swap.amount0_in);
        println!("     Amount1In: {}", swap.amount1_in);
        println!("     Amount0Out: {}", swap.amount0_out);
        println!("     Amount1Out: {}", swap.amount1_out);
    }
    
    // Print address balance changes
    println!("\n📋 Address Balance Changes ({}):", processed_tx.address_balance_changes.len());
    for (addr, changes) in &processed_tx.address_balance_changes {
        println!("  Address: {:?}", addr);
        println!("  Changes: {}", serde_json::to_string_pretty(changes)?);
    }
    
    // Expected addresses that should be involved (from the provided ProcessedTransaction)
    let expected_addresses = vec![
        "0x12ddA8BFfdbEBF79502B175fA1413a4765e69b2F",
        "0x62eD63Ce328665488914992B247b1EF8F78199A1", 
        "0xF2e24D564a08A3Acc31985eBD3C6Ee7FA9943a84",
        "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
        "0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e",
        "0x6D356ab697B0AB2A871B6Be79073A35664340440",
        "0x5423621C6D3E22465876deb92aD4f40Dc25b62A3",
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    ];
    
    // Expected ERC20 transfers count: 5 transfers
    let expected_erc20_transfers = 5;
    
    // Expected token contract
    let expected_token = Address::from_str("0xF2e24D564a08A3Acc31985eBD3C6Ee7FA9943a84")?;
    let expected_weth = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    
    println!("\n🔍 Validation Results:");
    
    // Check if we have the expected number of ERC20 transfers
    if processed_tx.erc20_transfers.len() >= expected_erc20_transfers {
        println!("✅ ERC20 transfers count: {} (expected at least {})", 
                 processed_tx.erc20_transfers.len(), expected_erc20_transfers);
    } else {
        println!("❌ ERC20 transfers count: {} (expected at least {})", 
                 processed_tx.erc20_transfers.len(), expected_erc20_transfers);
    }
    
    // Check if we have the expected token contract
    let has_expected_token = processed_tx.erc20_contracts.contains(&expected_token);
    if has_expected_token {
        println!("✅ Expected token contract found: {:?}", expected_token);
    } else {
        println!("❌ Expected token contract NOT found: {:?}", expected_token);
    }
    
    // Check for Uniswap swaps
    if processed_tx.uniswap_v2_swaps.len() >= 2 {
        println!("✅ Uniswap V2 swaps found: {}", processed_tx.uniswap_v2_swaps.len());
    } else {
        println!("❌ Expected at least 2 Uniswap V2 swaps, found: {}", processed_tx.uniswap_v2_swaps.len());
    }
    
    // Check transaction type
    if processed_tx.txn_type.contains("Swap") || processed_tx.txn_type.contains("Contract") {
        println!("✅ Transaction type correct: {}", processed_tx.txn_type);
    } else {
        println!("❌ Unexpected transaction type: {}", processed_tx.txn_type);
    }
    
    // Expected address balance changes with actual values (from your provided ProcessedTransaction)
    let expected_address_balance_changes = vec![
        (
            Address::from_str("0x6D356ab697B0AB2A871B6Be79073A35664340440")?, // from_address
            (-551826815374445.0, 0.01939941893084146) // (token_net, eth_net)
        ),
        (
            Address::from_str("0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e")?, // to_address
            (0.0, -0.01939941893084146) // no token change, negative ETH
        ),
        (
            Address::from_str("0xF2e24D564a08A3Acc31985eBD3C6Ee7FA9943a84")?, // token contract
            (-441461452299556.0, 0.0) // negative token change, no ETH
        ),
        (
            Address::from_str("0x5423621C6D3E22465876deb92aD4f40Dc25b62A3")?, // pool address
            (993288267674001.0, -0.044522467378667496) // positive token, negative ETH
        ),
    ];
    
    let mut state_value_checks_passed = 0;
    let mut total_state_checks = expected_address_balance_changes.len();
    
    for (addr, (expected_token_net, expected_eth_net)) in expected_address_balance_changes {
        if let Some(state_change) = processed_tx.address_balance_changes.get(&addr) {
            println!("✅ State change found for address: {:?}", addr);
            
            // Parse the state change JSON to get eth_net and token_net
            let eth_net = state_change.get("eth_net")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
                
            let token_net = if expected_token_net != 0.0 {
                // Look for the specific token in token_net
                let token_addr = "0xf2e24d564a08a3acc31985ebd3c6ee7fa9943a84";
                state_change.get("token_net")
                    .and_then(|tn| tn.as_object())
                    .and_then(|obj| obj.get(token_addr))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            
            // Check ETH values (with small tolerance for floating point)
            let eth_diff = (eth_net - expected_eth_net).abs();
            let eth_matches = eth_diff < 0.0000001;
            
            // Check token values (with small tolerance)
            let token_diff = (token_net - expected_token_net).abs();
            let token_matches = token_diff < 1.0; // Allow 1 unit difference for token amounts
            
            if eth_matches && token_matches {
                println!("   ✅ Values match: ETH {:.6}, Token {:.0}", eth_net, token_net);
                state_value_checks_passed += 1;
            } else {
                println!("   ❌ Values differ:");
                println!("      Expected: ETH {:.6}, Token {:.0}", expected_eth_net, expected_token_net);
                println!("      Actual:   ETH {:.6}, Token {:.0}", eth_net, token_net);
                println!("      Diff:     ETH {:.9}, Token {:.0}", eth_diff, token_diff);
            }
        } else {
            println!("⚠️  No state change found for address: {:?}", addr);
        }
    }
    
    println!("\n📊 Summary:");
    println!("State value checks passed: {}/{} expected addresses", state_value_checks_passed, total_state_checks);
    println!("ERC20 transfers: {}", processed_tx.erc20_transfers.len());
    println!("Internal transactions: {}", processed_tx.internal_transactions.len());
    println!("Uniswap V2 swaps: {}", processed_tx.uniswap_v2_swaps.len());
    println!("Unique addresses: {}", processed_tx.unique_addresses.len());
    
    if state_value_checks_passed == total_state_checks {
        println!("\n🎉 Test PASSED: All state change values match exactly!");
    } else if state_value_checks_passed >= 3 {
        println!("\n⚠️  Test PARTIAL: Most address balance changes match, some differences found");
    } else {
        println!("\n❌ Test FAILED: State change values don't match expected results");
    }
    
    Ok(())
}