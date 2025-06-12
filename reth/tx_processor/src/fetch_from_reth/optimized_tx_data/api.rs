//! Main API for optimized transaction data retrieval

use anyhow::Result;
use std::time::Instant;
use std::str::FromStr;
use std::collections::HashMap;

use ethers_core::types::H256;
use revm_primitives::{Address as RevmAddress, U256 as RevmU256, Log as RevmLog, B256};

use super::types::{
    BasicTxData, SmartTxData, FullTxData, TransactionDataOptions, 
    DataLevel, PerformanceMetrics, Erc20Transfer, InternalTransfer,
};
use super::heuristics::{
    detect_transaction_type, should_use_simulation,
    get_optimization_recommendation,
};
use crate::fetch_from_reth::{RethDatabaseProvider, RethDataProvider};
use crate::simulate_signed_tx::{simulate_signed_tx, simulate_signed_tx_from_db};

/// Get basic transaction data from database only (fastest ~1ms)
pub async fn get_basic_transaction_data(
    tx_hash: H256,
    options: TransactionDataOptions,
) -> Result<BasicTxData> {
    get_basic_transaction_data_with_provider(tx_hash, options, None).await
}

/// Get basic transaction data with optional pre-created provider (performance optimized)
pub async fn get_basic_transaction_data_with_provider(
    tx_hash: H256,
    options: TransactionDataOptions,
    provider: Option<&RethDatabaseProvider>,
) -> Result<BasicTxData> {
    let start_time = Instant::now();
    
    // Convert H256 to B256 for database lookup
    let tx_hash_str = format!("{:x}", tx_hash);
    let tx_hash_b256 = B256::from_str(&tx_hash_str)?;
    
    // Use provided provider or create new one
    let db_provider = if let Some(provider) = provider {
        // Use existing provider (fast path)
        provider
    } else {
        // Create new provider (slow path - opens database)
        let datadir = options.reth_datadir.as_deref()
            .unwrap_or("/home/nima/.local/share/reth/mainnet");
        let _owned_provider = RethDatabaseProvider::new(datadir)?;
        // We need to return a reference, but we own this provider
        // For now, we'll use the slow path when no provider is given
        return get_basic_transaction_data_slow_path(tx_hash, options).await;
    };
    
    // Fetch transaction data from database
    let tx_data = db_provider.fetch_transaction(tx_hash_b256)?;
    
    // Continue with common processing logic
    build_basic_tx_data(tx_hash, tx_data, start_time)
}

/// Slow path for when no provider is given (creates new provider)
async fn get_basic_transaction_data_slow_path(
    tx_hash: H256,
    options: TransactionDataOptions,
) -> Result<BasicTxData> {
    let start_time = Instant::now();
    
    // Convert H256 to B256 for database lookup
    let tx_hash_str = format!("{:x}", tx_hash);
    let tx_hash_b256 = B256::from_str(&tx_hash_str)?;
    
    // Connect to Reth database (slow - opens database)
    let datadir = options.reth_datadir.as_deref()
        .unwrap_or("/home/nima/.local/share/reth/mainnet");
    let db_provider = RethDatabaseProvider::new(datadir)?;
    
    // Fetch transaction data from database
    let tx_data = db_provider.fetch_transaction(tx_hash_b256)?;
    
    // Continue with common processing logic
    build_basic_tx_data(tx_hash, tx_data, start_time)
}

/// Common logic to build BasicTxData from fetched transaction data
fn build_basic_tx_data(
    tx_hash: H256,
    tx_data: crate::fetch_from_reth::TransactionData,
    start_time: Instant,
) -> Result<BasicTxData> {
    
    // Extract ERC20 transfers from logs
    let erc20_transfers = extract_erc20_transfers_from_logs(&tx_data.logs);
    
    // Detect transaction type for optimization hints
    let _transaction_type = detect_transaction_type(
        tx_data.to,
        tx_data.value,
        &tx_data.input,
        tx_data.gas_limit,
    );
    
    let database_time = start_time.elapsed().as_secs_f64() * 1000.0;
    
    let performance = PerformanceMetrics {
        data_source: "database_only".to_string(),
        retrieval_time_ms: database_time,
        database_time_ms: Some(database_time),
        simulation_time_ms: None,
        data_level: "basic".to_string(),
        optimization_applied: true,
        internal_transfers_found: None,
    };
    
    Ok(BasicTxData {
        hash: tx_hash,
        from: tx_data.from,
        to: tx_data.to,
        value: tx_data.value,
        gas_limit: tx_data.gas_limit,
        gas_price: tx_data.gas_price,
        gas_used: tx_data.gas_used,
        nonce: tx_data.nonce,
        input_data: tx_data.input.to_vec(),
        block_number: tx_data.block_number,
        block_hash: H256::from_slice(&tx_data.block_hash[..]),
        transaction_index: tx_data.transaction_index,
        status: tx_data.receipt_status,
        logs: tx_data.logs.clone(),
        log_count: tx_data.logs.len(),
        is_contract_call: tx_data.to.is_some() && !tx_data.input.is_empty(),
        erc20_transfers,
        performance,
    })
}

/// Get smart transaction data with conditional simulation
pub async fn get_smart_transaction_data(
    tx_hash: H256,
    options: TransactionDataOptions,
) -> Result<SmartTxData> {
    get_smart_transaction_data_with_provider(tx_hash, options, None).await
}

/// Get smart transaction data with optional pre-created provider (performance optimized)
pub async fn get_smart_transaction_data_with_provider(
    tx_hash: H256,
    options: TransactionDataOptions,
    provider: Option<&RethDatabaseProvider>,
) -> Result<SmartTxData> {
    let start_time = Instant::now();
    
    // First get basic data (fast) - use provided provider if available
    let basic_data = get_basic_transaction_data_with_provider(tx_hash, options.clone(), provider).await?;
    
    // Detect transaction type
    let transaction_type = detect_transaction_type(
        basic_data.to,
        basic_data.value,
        &basic_data.input_data,
        basic_data.gas_limit,
    );
    
    // Decide if simulation is needed
    let needs_simulation = should_use_simulation(&transaction_type, &options);
    
    let (internal_transfers, simulation_performed, total_time) = if needs_simulation {
        // Smart mode: Simulation needed for this transaction type
        
        let sim_start = Instant::now();
        let simulation_result = if options.reth_datadir.is_some() {
            simulate_signed_tx_from_db(tx_hash, options.reth_datadir.as_deref(), &options.rpc_url).await?
        } else {
            simulate_signed_tx(tx_hash, &options.rpc_url).await?
        };
        let _sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
        
        let internal_transfers: Vec<InternalTransfer> = simulation_result.internal_transfers.iter()
            .map(|transfer| InternalTransfer {
                from: transfer.from,
                to: transfer.to,
                value: transfer.value,
                call_type: "CALL".to_string(), // Simplified
                depth: 0, // Simplified
            })
            .collect();
        
        let total_time = start_time.elapsed().as_secs_f64() * 1000.0;
        
        (Some(internal_transfers), true, total_time)
    } else {
        // Smart mode: Database-only sufficient for this transaction type
        let total_time = start_time.elapsed().as_secs_f64() * 1000.0;
        (None, false, total_time)
    };
    
    // Analyze complexity
    let complex_interactions = basic_data.gas_used > 200_000 || basic_data.log_count > 10;
    let dex_swaps_detected = count_dex_swaps(&basic_data.logs);
    let multi_hop_detected = dex_swaps_detected > 1;
    
    // Update performance metrics
    let mut updated_basic = basic_data;
    updated_basic.performance = PerformanceMetrics {
        data_source: if simulation_performed { "database_plus_simulation" } else { "database_only" }.to_string(),
        retrieval_time_ms: total_time,
        database_time_ms: Some(updated_basic.performance.database_time_ms.unwrap()),
        simulation_time_ms: if simulation_performed { Some(total_time - updated_basic.performance.database_time_ms.unwrap()) } else { None },
        data_level: "smart".to_string(),
        optimization_applied: !simulation_performed, // Optimization applied if we skipped simulation
        internal_transfers_found: internal_transfers.as_ref().map(|it| it.len()),
    };
    
    Ok(SmartTxData {
        basic: updated_basic,
        internal_transfers,
        simulation_performed,
        transaction_type,
        complex_interactions,
        dex_swaps_detected,
        multi_hop_detected,
    })
}

/// Get full transaction analysis with complete simulation (slowest but complete)
pub async fn get_full_transaction_analysis(
    tx_hash: H256,
    options: TransactionDataOptions,
) -> Result<FullTxData> {
    get_full_transaction_analysis_with_provider(tx_hash, options, None).await
}

/// Get full transaction analysis with optional pre-created provider (performance optimized)
pub async fn get_full_transaction_analysis_with_provider(
    tx_hash: H256,
    options: TransactionDataOptions,
    provider: Option<&RethDatabaseProvider>,
) -> Result<FullTxData> {
    let start_time = Instant::now();
    
    // Get smart data first - use provided provider if available
    let mut smart_options = options.clone();
    smart_options.force_level = Some(DataLevel::Smart);
    let smart_data = get_smart_transaction_data_with_provider(tx_hash, smart_options, provider).await?;
    
    // Always perform full simulation
    // Full mode: Always performing complete simulation
    
    let sim_start = Instant::now();
    let simulation_result = if options.reth_datadir.is_some() {
        simulate_signed_tx_from_db(tx_hash, options.reth_datadir.as_deref(), &options.rpc_url).await?
    } else {
        simulate_signed_tx(tx_hash, &options.rpc_url).await?
    };
    let sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
    
    // Convert internal transfers
    let internal_transfers: Vec<InternalTransfer> = simulation_result.internal_transfers.iter()
        .map(|transfer| InternalTransfer {
            from: transfer.from,
            to: transfer.to,
            value: transfer.value,
            call_type: "CALL".to_string(), // Simplified
            depth: 0, // Simplified
        })
        .collect();
    
    // Calculate state changes (simplified)
    let addresses_affected = calculate_addresses_affected(&internal_transfers, &simulation_result.logs);
    let eth_movements = calculate_eth_movements(&internal_transfers);
    let token_movements = calculate_token_movements(&simulation_result.logs);
    
    let total_time = start_time.elapsed().as_secs_f64() * 1000.0;
    
    // Update performance metrics
    let mut updated_smart = smart_data;
    updated_smart.basic.performance = PerformanceMetrics {
        data_source: "complete_simulation".to_string(),
        retrieval_time_ms: total_time,
        database_time_ms: updated_smart.basic.performance.database_time_ms,
        simulation_time_ms: Some(sim_time),
        data_level: "complete".to_string(),
        optimization_applied: false, // No optimization in complete mode
        internal_transfers_found: Some(internal_transfers.len()),
    };
    
    Ok(FullTxData {
        smart: updated_smart,
        internal_transfers,
        call_trace: None, // Could be added later
        gas_refunded: simulation_result.gas_refunded,
        output_data: simulation_result.output_data.to_vec(),
        simulation_success: true, // Simplified - we got a result
        revert_reason: None, // Could be extracted from simulation
        addresses_affected,
        eth_movements,
        token_movements,
    })
}

/// Extract ERC20 transfers from transaction logs
fn extract_erc20_transfers_from_logs(logs: &[RevmLog]) -> Vec<Erc20Transfer> {
    let mut transfers = Vec::new();
    
    // ERC20 Transfer event signature
    // ERC20 Transfer event signature: Transfer(address,address,uint256)
    
    let erc20_transfer_topic = revm_primitives::keccak256("Transfer(address,address,uint256)".as_bytes());
    
    for (log_index, log) in logs.iter().enumerate() {
        if log.topics().len() >= 3 && log.topics()[0] == erc20_transfer_topic {
            if log.topics()[1].as_slice().len() >= 20 && log.topics()[2].as_slice().len() >= 20 {
                let from = RevmAddress::from_slice(&log.topics()[1].as_slice()[12..]);
                let to = RevmAddress::from_slice(&log.topics()[2].as_slice()[12..]);
                
                // Extract amount from data
                let amount = if log.data.data.len() >= 32 {
                    RevmU256::from_be_bytes(log.data.data[0..32].try_into().unwrap_or([0u8; 32]))
                } else {
                    RevmU256::ZERO
                };
                
                transfers.push(Erc20Transfer {
                    token_address: log.address,
                    from,
                    to,
                    amount,
                    log_index,
                });
            }
        }
    }
    
    transfers
}

/// Count DEX swaps in logs (simplified heuristic)
fn count_dex_swaps(logs: &[RevmLog]) -> usize {
    // Look for Uniswap Swap events and other DEX patterns
    let swap_signatures = [
        revm_primitives::keccak256("Swap(address,uint256,uint256,uint256,uint256,address)".as_bytes()), // Uniswap V2
        revm_primitives::keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)".as_bytes()), // Uniswap V3
    ];
    
    logs.iter()
        .filter(|log| {
            !log.topics().is_empty() && 
            swap_signatures.iter().any(|&sig| sig == log.topics()[0])
        })
        .count()
}

/// Calculate addresses affected by the transaction
fn calculate_addresses_affected(internal_transfers: &[InternalTransfer], logs: &[RevmLog]) -> usize {
    let mut addresses = std::collections::HashSet::new();
    
    // Add addresses from internal transfers
    for transfer in internal_transfers {
        addresses.insert(transfer.from);
        addresses.insert(transfer.to);
    }
    
    // Add addresses from logs
    for log in logs {
        addresses.insert(log.address);
        for topic in log.topics() {
            if topic.as_slice().len() >= 20 {
                // Try to extract address from topic (for indexed address parameters)
                let potential_addr = RevmAddress::from_slice(&topic.as_slice()[12..]);
                addresses.insert(potential_addr);
            }
        }
    }
    
    addresses.len()
}

/// Calculate ETH movements from internal transfers
fn calculate_eth_movements(internal_transfers: &[InternalTransfer]) -> HashMap<RevmAddress, i128> {
    let mut movements = HashMap::new();
    
    for transfer in internal_transfers {
        let amount = transfer.value.try_into().unwrap_or(0i128);
        
        // Subtract from sender
        *movements.entry(transfer.from).or_insert(0) -= amount;
        
        // Add to receiver
        *movements.entry(transfer.to).or_insert(0) += amount;
    }
    
    // Remove zero movements
    movements.retain(|_, &mut balance| balance != 0);
    movements
}

/// Calculate token movements from logs (simplified)
fn calculate_token_movements(logs: &[RevmLog]) -> HashMap<RevmAddress, HashMap<RevmAddress, i128>> {
    let mut movements: HashMap<RevmAddress, HashMap<RevmAddress, i128>> = HashMap::new();
    
    let erc20_transfer_topic = revm_primitives::keccak256("Transfer(address,address,uint256)".as_bytes());
    
    for log in logs {
        if log.topics().len() >= 3 && log.topics()[0] == erc20_transfer_topic {
            if log.topics()[1].as_slice().len() >= 20 && log.topics()[2].as_slice().len() >= 20 {
                let token_address = log.address;
                let from = RevmAddress::from_slice(&log.topics()[1].as_slice()[12..]);
                let to = RevmAddress::from_slice(&log.topics()[2].as_slice()[12..]);
                
                let amount = if log.data.data.len() >= 32 {
                    let amount_u256 = RevmU256::from_be_bytes(log.data.data[0..32].try_into().unwrap_or([0u8; 32]));
                    amount_u256.try_into().unwrap_or(0i128)
                } else {
                    0i128
                };
                
                let token_movements = movements.entry(token_address).or_insert_with(HashMap::new);
                
                // Subtract from sender
                *token_movements.entry(from).or_insert(0) -= amount;
                
                // Add to receiver
                *token_movements.entry(to).or_insert(0) += amount;
            }
        }
    }
    
    // Remove zero movements
    for token_movements in movements.values_mut() {
        token_movements.retain(|_, &mut balance| balance != 0);
    }
    movements.retain(|_, token_movements| !token_movements.is_empty());
    
    movements
}

/// Get optimization recommendation for a transaction
pub async fn get_transaction_optimization_advice(
    tx_hash: H256,
    rpc_url: &str,
) -> Result<String> {
    let options = TransactionDataOptions {
        rpc_url: rpc_url.to_string(),
        ..Default::default()
    };
    
    let basic_data = get_basic_transaction_data(tx_hash, options).await?;
    
    let transaction_type = detect_transaction_type(
        basic_data.to,
        basic_data.value,
        &basic_data.input_data,
        basic_data.gas_limit,
    );
    
    Ok(get_optimization_recommendation(
        &transaction_type,
        basic_data.gas_used,
        basic_data.log_count,
    ))
}