/// Fast RPC-Based Transaction Simulator using debug_traceCall
/// 
/// This module provides high-performance transaction simulation by leveraging
/// the node's debug_traceCall RPC method. It's the recommended approach for
/// production mempool monitoring due to its superior performance (~5ms).
///
/// IMPORTANT: This simulator returns RAW state changes - it does NOT treat WETH
/// and ETH as the same. WETH transfers are tracked as ERC20 token changes.
/// Use debug_tracecall_state_diff_calculator if you need WETH=ETH logic.
///
/// Key Features:
/// - 10x faster than REVM-based simulation (~5ms vs ~40-50ms)
/// - Uses node's existing state and execution engine
/// - Supports pending transaction simulation without waiting for receipts
/// - Extracts logs and state changes from trace results
///
/// Requirements:
/// - Node must support debug_traceCall with callTracer
/// - Recommended: Local node for best performance
///
/// This is the production choice for real-time mempool analysis where
/// speed is critical and the node's trace data is sufficient.

use ethers::types::{Transaction, H256, U256, Address as EthersAddress};
use ethers::providers::{Http, Provider, Middleware};
use eyre::Result;
use std::collections::HashMap;
use std::str::FromStr;
use revm_primitives::Address as RevmAddress;
use revm_tx_simulator_lib::process_tx::state_diff_utils::{CalculatedAccountChanges, SignedAmount, AccountMovements};
use crate::mempool_fetcher::types::TransactionView;
use crate::tx_simulator::debug_tracecall_state_diff_calculator::{
    DebugTraceCallStateDiffCalculator, EthTransfer, Erc20Transfer
};
use crate::common::address::checksum_address;
use revm_context::BlockEnv;
use hex;

/// debug_traceCall-based transaction simulator
/// Achieves ~5ms simulation time using debug_traceCall
pub struct DebugTraceCallSimulator {
    provider: Provider<Http>,
}

impl DebugTraceCallSimulator {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        
        Ok(Self {
            provider,
        })
    }
    
    /// Process a transaction and return account changes with checksummed addresses as keys
    pub async fn process_transaction(
        &self,
        tx_view: &TransactionView,
        _block_env: &BlockEnv, // Not needed for RPC simulation
    ) -> Result<Option<HashMap<String, CalculatedAccountChanges>>> {
        // Convert TransactionView to ethers Transaction for RPC call
        let tx = convert_transaction_view_to_ethers(tx_view)?;
        
        // Create call request for debug_traceCall
        let call_request = serde_json::json!({
            "from": format!("{:#x}", tx.from),
            "to": tx.to.map(|addr| format!("{:#x}", addr)),
            "value": format!("{:#x}", tx.value),
            "data": format!("0x{}", hex::encode(&tx.input)),
            "gas": format!("{:#x}", tx.gas),
            "gasPrice": format!("{:#x}", tx.gas_price.unwrap_or_default())
        });
        
        // Call debug_traceCall with callTracer to get logs
        let trace_result: serde_json::Value = self.provider.request(
            "debug_traceCall",
            (call_request, "latest", serde_json::json!({"tracer": "callTracer", "tracerConfig": {"withLog": true}}))
        ).await?;
        
        // Check if the trace was successful
        if let Some(error) = trace_result.get("error") {
            // Don't log transaction failures - these are expected for invalid transactions
            // Only debug log for our own debugging if needed
            let error_str = error.as_str().unwrap_or("");
            tracing::debug!("Transaction would fail on-chain: {}", error_str);
            return Ok(None);
        }
        
        // Parse logs from trace result
        let mut eth_transfers = Vec::new();
        let mut erc20_transfers = Vec::new();
        
        // Helper function to parse logs from any level of the trace
        fn parse_logs_recursive(
            trace: &serde_json::Value,
            eth_transfers: &mut Vec<EthTransfer>,
            erc20_transfers: &mut Vec<Erc20Transfer>,
            depth: u32,
        ) {
            // Parse logs at this level for ERC20 transfers
            if let Some(logs) = trace.get("logs").and_then(|l| l.as_array()) {
                let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
                
                for (log_index, log_entry) in logs.iter().enumerate() {
                    if let (Some(topics), Some(data), Some(address)) = (
                        log_entry.get("topics").and_then(|t| t.as_array()),
                        log_entry.get("data").and_then(|d| d.as_str()),
                        log_entry.get("address").and_then(|a| a.as_str())
                    ) {
                        if topics.len() >= 3 && topics[0].as_str().unwrap_or("") == transfer_topic {
                            let from_hex = topics[1].as_str().unwrap_or("0x0000000000000000000000000000000000000000");
                            let to_hex = topics[2].as_str().unwrap_or("0x0000000000000000000000000000000000000000");
                            
                            // Parse addresses from topic (last 40 chars)
                            let from_addr: EthersAddress = format!("0x{}", &from_hex[26..]).parse().unwrap_or_default();
                            let to_addr: EthersAddress = format!("0x{}", &to_hex[26..]).parse().unwrap_or_default();
                            let token_addr: EthersAddress = address.parse().unwrap_or_default();
                            
                            if let Ok(amount_bytes) = hex::decode(data.trim_start_matches("0x")) {
                                let amount_raw = U256::from_big_endian(&amount_bytes);
                                
                                erc20_transfers.push(Erc20Transfer {
                                    token_address: token_addr,
                                    from_address: from_addr,
                                    to_address: to_addr,
                                    amount: amount_raw.as_u128() as f64,
                                    log_index: (depth * 1000 + log_index as u32) as u64,
                                });
                            }
                        }
                    }
                }
            }
            
            // Check for ETH transfers in this call
            if let (Some(from), Some(to), Some(value)) = (
                trace.get("from").and_then(|f| f.as_str()),
                trace.get("to").and_then(|t| t.as_str()),
                trace.get("value").and_then(|v| v.as_str())
            ) {
                if let (Ok(from_addr), Ok(to_addr)) = (from.parse::<EthersAddress>(), to.parse::<EthersAddress>()) {
                    if let Ok(value_u256) = U256::from_str_radix(value.trim_start_matches("0x"), 16) {
                        if !value_u256.is_zero() {
                            let value_eth = value_u256.as_u128() as f64 / 1e18;
                            eth_transfers.push(EthTransfer {
                                from_address: from_addr,
                                to_address: to_addr,
                                amount: value_eth,
                                log_index: Some(999990 + depth as u64),
                                depth: Some(depth as u64),
                            });
                        }
                    }
                }
            }
            
            // Recursively parse internal calls
            if let Some(calls) = trace.get("calls").and_then(|c| c.as_array()) {
                for call in calls {
                    parse_logs_recursive(call, eth_transfers, erc20_transfers, depth + 1);
                }
            }
        }
        
        // Add top-level value transfer if any
        if !tx.value.is_zero() {
            let value_eth = tx.value.as_u128() as f64 / 1e18;
            if let Some(to_addr) = tx.to {
                eth_transfers.push(EthTransfer {
                    from_address: tx.from,
                    to_address: to_addr,
                    amount: value_eth,
                    log_index: Some(999998),
                    depth: Some(0),
                });
            }
        }
        
        // Parse all logs and transfers recursively
        parse_logs_recursive(&trace_result, &mut eth_transfers, &mut erc20_transfers, 0);
        
        // Use state calculator to process transfers
        let mut calculator = DebugTraceCallStateDiffCalculator::default();
        
        // Get current block number (using dummy value for mempool tx)
        let block_number = self.provider.get_block_number().await?.as_u64();
        
        let address_changes = calculator.calculate_state_changes_from_transfers(
            tx.from,
            block_number,
            0, // tx index
            &eth_transfers,
            &erc20_transfers,
        )?;
        
        // Convert from AddressStateChange to CalculatedAccountChanges format
        let mut state_changes: HashMap<String, CalculatedAccountChanges> = HashMap::new();
        
        for (addr, changes) in address_changes {
            // Convert ethers Address to revm Address
            let revm_addr = RevmAddress::from_slice(addr.as_bytes());
            // Convert to checksummed address for pool cache compatibility
            let addr_hex = format!("0x{}", hex::encode(addr.as_bytes()));
            let addr_checksummed = checksum_address(&addr_hex);
            
            // Convert ETH changes
            let eth_net_change = if changes.eth_net < 0.0 {
                SignedAmount {
                    absolute_value: revm_primitives::U256::from(((-changes.eth_net * 1e18) as u128)),
                    is_negative: true,
                }
            } else {
                SignedAmount {
                    absolute_value: revm_primitives::U256::from(((changes.eth_net * 1e18) as u128)),
                    is_negative: false,
                }
            };
            
            // Convert token changes - we need to handle the string keys
            let mut token_net_changes = HashMap::new();
            for (token_key, amount) in changes.token_net {
                // Try to parse as address, skip if it's a symbol
                if let Ok(token_addr) = token_key.parse::<EthersAddress>() {
                    let revm_token_addr = RevmAddress::from_slice(token_addr.as_bytes());
                    let signed_amount = if amount < 0.0 {
                        SignedAmount {
                            absolute_value: revm_primitives::U256::from(((-amount) as u128)),
                            is_negative: true,
                        }
                    } else {
                        SignedAmount {
                            absolute_value: revm_primitives::U256::from((amount as u128)),
                            is_negative: false,
                        }
                    };
                    token_net_changes.insert(revm_token_addr, signed_amount);
                }
            }
            
            let account_changes = CalculatedAccountChanges {
                address: revm_addr,
                eth_net_change,
                token_net_changes,
                token_infos: Vec::new(),
                movements: AccountMovements::default(),
            };
            
            state_changes.insert(addr_checksummed, account_changes);
        }
        
        if state_changes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(state_changes))
        }
    }
}

/// Convert TransactionView to ethers Transaction
fn convert_transaction_view_to_ethers(tx_view: &TransactionView) -> Result<Transaction> {
    let mut tx = Transaction::default();
    
    // Set transaction hash
    if tx_view.hash.len() == 32 {
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&tx_view.hash);
        tx.hash = H256::from(hash_bytes);
    }
    
    // Set from address
    if tx_view.from.len() == 20 {
        let mut from_bytes = [0u8; 20];
        from_bytes.copy_from_slice(&tx_view.from);
        tx.from = ethers::types::Address::from(from_bytes);
    }
    
    // Set to address
    if let Some(to_bytes) = &tx_view.to {
        if to_bytes.len() == 20 {
            let mut to_addr = [0u8; 20];
            to_addr.copy_from_slice(to_bytes);
            tx.to = Some(ethers::types::Address::from(to_addr));
        }
    }
    
    // Set value
    tx.value = tx_view.value;
    
    // Set gas price
    tx.gas_price = tx_view.gas_price;
    
    // Set gas limit
    tx.gas = tx_view.gas_limit.unwrap_or_else(|| U256::from(21000));
    
    // Set nonce
    tx.nonce = tx_view.nonce.unwrap_or_else(U256::zero);
    
    // Set input data
    if let Some(input_data) = &tx_view.input_data {
        tx.input = ethers::types::Bytes::from(input_data.clone());
    }
    
    Ok(tx)
}

/// Parse hex string to U256
fn parse_hex_u256(hex_str: &str) -> U256 {
    let hex_str = hex_str.trim_start_matches("0x");
    U256::from_str_radix(hex_str, 16).unwrap_or_default()
}