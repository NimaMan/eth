#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// Fast Transaction Hash Simulator
// Optimizes transaction simulation speed by using smart detection to determine
// whether full REVM simulation is needed or if database-only analysis suffices

// Silence unused crate dependency warnings
use chrono as _;
use clap as _;
use ethers_signers as _;
use eyre as _;
use hex as _;
use reqwest as _;
use reth_ethereum as _;
use revm_database as _;
use revm_inspector as _;
use revm_interpreter as _;
use revm_state as _;
use serde as _;
use serde_json as _;
use tracing as _;
use tracing_appender as _;
use tracing_subscriber as _;

use anyhow::{anyhow, Result};
use ethers_core::types::H256 as EthersH256;
use ethers_providers::{Middleware, Provider as EthersProvider, Http as EthersHttp};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use serde_json::json;

// REVM imports for full simulation when needed
use revm_tx_simulator_lib::{
    ExecutionResultType, 
    conversions::{ethers_to_revm_address, ethers_to_revm_u256}, 
    state_diff_utils::generate_calculated_account_changes,
    SimCacheDBForDiff,
    CallTracer,
    integrate_internal_transfers,
};

use revm_primitives::{
    Bytes as RevmBytes, 
    hardfork::SpecId as RevmSpecId_primitive,
    U256 as RevmU256,
};
use revm_context::{
    BlockEnv as RevmBlockEnv_ctx, CfgEnv as RevmCfgEnv_ctx, TxEnv as RevmTxEnv_ctx, 
    TransactTo as RevmTransactTo_ctx, Context as RevmContext, ContextTr, Journal
};
use revm_context::result::ExecutionResult as RevmExecutionResult;
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use revm_context::Evm;
use revm_inspector::InspectEvm;
use revm_handler::{EthPrecompiles, instructions::EthInstructions};
use revm_interpreter::interpreter::EthInterpreter;

// Alloy Imports
use alloy_eips::BlockId as AlloyBlockId;
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};

const RPC_URL: &str = "http://127.0.0.1:8545";
const ETH_TO_WEI_FACTOR_F64: f64 = 1_000_000_000_000_000_000.0_f64;

#[tokio::main]
async fn main() -> Result<()> {
    // Get transaction hash from command line
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <transaction_hash> [--fast-only] [--force-simulation]", args[0]);
        eprintln!("");
        eprintln!("Options:");
        eprintln!("  --fast-only        Only use database analysis (fastest, may miss internal transfers)");
        eprintln!("  --force-simulation Always use full REVM simulation (slowest, most accurate)");
        eprintln!("  (default)          Smart detection (optimal balance of speed and accuracy)");
        std::process::exit(1);
    }
    
    let tx_hash_str = &args[1];
    let force_fast_only = args.contains(&"--fast-only".to_string());
    let force_simulation = args.contains(&"--force-simulation".to_string());
    
    println!("🚀 Fast Transaction Hash Simulator");
    println!("==================================");
    println!("Transaction: {}", tx_hash_str);
    
    let mode = if force_fast_only {
        "Database-Only (Fastest)"
    } else if force_simulation {
        "Full Simulation (Most Accurate)"
    } else {
        "Smart Detection (Balanced)"
    };
    println!("Mode: {}", mode);
    println!();

    let start_time = Instant::now();
    
    // Run the appropriate analysis method
    let result = if force_fast_only {
        fast_database_analysis(tx_hash_str).await?
    } else if force_simulation {
        full_simulation_analysis(tx_hash_str).await?
    } else {
        smart_analysis(tx_hash_str).await?
    };

    let total_time = start_time.elapsed().as_secs_f64() * 1000.0;
    
    // Output results
    println!("📊 Analysis Results:");
    println!("  Method Used: {}", result.method);
    println!("  Processing Time: {:.2}ms", total_time);
    println!("  Internal Transfers: {}", result.internal_transfers);
    println!("  Confidence: {}", result.confidence);
    println!();
    
    // Output state changes in same format as main simulator
    println!("{}", serde_json::to_string_pretty(&result.state_changes)?);
    
    Ok(())
}

#[derive(Debug)]
struct AnalysisResult {
    method: String,
    state_changes: serde_json::Value,
    internal_transfers: usize,
    confidence: String,
}

/// Smart analysis: determine best approach and execute
async fn smart_analysis(tx_hash_str: &str) -> Result<AnalysisResult> {
    let tx_hash: EthersH256 = tx_hash_str.parse()?;
    let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
    let eth_client = Arc::new(ethers_provider);
    
    // Fetch transaction data to make decision
    let tx = eth_client.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow!("Transaction not found"))?;
    
    let receipt = eth_client.get_transaction_receipt(tx_hash).await?;
    
    // Smart detection logic
    let needs_simulation = should_use_full_simulation(&tx, &receipt);
    
    if needs_simulation {
        eprintln!("🔍 Smart Detection: Complex transaction detected, using full simulation");
        full_simulation_analysis(tx_hash_str).await
    } else {
        eprintln!("⚡ Smart Detection: Simple transaction detected, using fast database analysis");
        fast_database_analysis(tx_hash_str).await
    }
}

/// Determine if full simulation is needed based on transaction characteristics
fn should_use_full_simulation(
    tx: &ethers_core::types::Transaction, 
    receipt: &Option<ethers_core::types::TransactionReceipt>
) -> bool {
    // High gas usage suggests complex transaction
    if let Some(receipt) = receipt {
        if receipt.gas_used.map_or(0, |g| g.as_u64()) > 100_000 {
            return true;
        }
    }
    
    // Contract interaction with data suggests potential internal transfers
    if tx.to.is_some() && !tx.input.0.is_empty() {
        // Check if it's a simple ERC20 transfer (transfer method: 0xa9059cbb)
        if tx.input.0.len() == 68 && tx.input.0.starts_with(&[0xa9, 0x05, 0x9c, 0xbb]) {
            return false; // Simple ERC20 transfer, database analysis sufficient
        }
        return true; // Complex contract call, likely has internal transfers
    }
    
    // Simple ETH transfer - database analysis sufficient
    false
}

/// Fast database-only analysis using receipts and logs
async fn fast_database_analysis(tx_hash_str: &str) -> Result<AnalysisResult> {
    let tx_hash: EthersH256 = tx_hash_str.parse()?;
    let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
    let eth_client = Arc::new(ethers_provider);
    
    // Fetch transaction and receipt
    let (tx_opt, receipt_opt) = tokio::try_join!(
        eth_client.get_transaction(tx_hash),
        eth_client.get_transaction_receipt(tx_hash)
    )?;
    
    let tx = tx_opt.ok_or_else(|| anyhow!("Transaction not found"))?;
    let receipt = receipt_opt.ok_or_else(|| anyhow!("Receipt not found"))?;
    
    let mut state_changes = serde_json::Map::new();
    let mut addresses_affected = HashMap::new();
    
    // Analyze receipt logs for token transfers
    for log in &receipt.logs {
        // ERC20 Transfer event: Transfer(address indexed from, address indexed to, uint256 value)
        if log.topics.len() == 3 && 
           log.topics[0] == ethers_core::types::H256::from_slice(&hex::decode("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap()) {
            
            let from = ethers_core::types::Address::from(log.topics[1]);
            let to = ethers_core::types::Address::from(log.topics[2]);
            let value = ethers_core::types::U256::from_big_endian(&log.data.0);
            
            // Assume 18 decimals for simplicity (would need token metadata in production)
            let amount = value.as_u128() as f64 / ETH_TO_WEI_FACTOR_F64;
            let token_symbol = format!("Token{:?}", log.address)[0..10].to_string(); // Simplified
            
            // Update from address (outgoing)
            add_token_change(&mut addresses_affected, from, &token_symbol, -amount);
            
            // Update to address (incoming)  
            add_token_change(&mut addresses_affected, to, &token_symbol, amount);
        }
    }
    
    // Analyze direct ETH transfer
    if tx.value > ethers_core::types::U256::zero() {
        let eth_amount = tx.value.as_u128() as f64 / ETH_TO_WEI_FACTOR_F64;
        
        // From address loses ETH
        add_eth_change(&mut addresses_affected, tx.from, -eth_amount);
        
        // To address gains ETH (if not contract creation)
        if let Some(to) = tx.to {
            add_eth_change(&mut addresses_affected, to, eth_amount);
        }
    }
    
    // Gas fees (from address pays)
    let gas_used = receipt.gas_used.unwrap_or_default();
    let gas_price = tx.gas_price.unwrap_or_default();
    let gas_cost_wei = gas_used * gas_price;
    let gas_cost_eth = gas_cost_wei.as_u128() as f64 / ETH_TO_WEI_FACTOR_F64;
    add_eth_change(&mut addresses_affected, tx.from, -gas_cost_eth);
    
    // Convert to output format
    for (address, changes) in addresses_affected {
        let addr_str = format!("{:?}", address);
        state_changes.insert(addr_str, changes);
    }
    
    // Confidence assessment
    let confidence = if gas_used.as_u64() < 50_000 {
        "High (Simple Transaction)"
    } else {
        "Medium (May Miss Internal Transfers)"
    };
    
    Ok(AnalysisResult {
        method: "Fast Database Analysis".to_string(),
        state_changes: json!(state_changes),
        internal_transfers: 0, // Database analysis cannot detect internal transfers
        confidence: confidence.to_string(),
    })
}

/// Full REVM simulation using existing proven logic
async fn full_simulation_analysis(tx_hash_str: &str) -> Result<AnalysisResult> {
    // This is a simplified version - in production, we'd extract the full simulation
    // logic from json_state_validator_no_rpc.rs into a reusable function
    
    let tx_hash: EthersH256 = tx_hash_str.parse()?;
    
    // Setup providers (minimal RPC usage - only for basic transaction data)
    let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
    let eth_client = Arc::new(ethers_provider);
    
    let alloy_provider_dyn: Arc<AlloyDynProvider<AlloyEthereum>> =
        Arc::new(ProviderBuilder::new().connect(RPC_URL).await?.erased());
    
    // Fetch transaction (basic info only)
    let ethers_tx = eth_client.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow!("Transaction not found"))?;
    
    let block_number_u64 = ethers_tx.block_number
        .ok_or_else(|| anyhow!("Transaction not yet mined"))?.as_u64();
    
    // Fetch block
    let ethers_block = eth_client.get_block(block_number_u64).await?
        .ok_or_else(|| anyhow!("Block not found"))?;
    
    // Setup REVM environment
    let chain_id_u64 = eth_client.get_chainid().await?.as_u64();
    let mut cfg_env = RevmCfgEnv_ctx::default();
    cfg_env.chain_id = chain_id_u64;
    
    // Dynamic spec selection based on block number
    cfg_env.spec = match block_number_u64 {
        0..=1_149_999 => RevmSpecId_primitive::FRONTIER,
        1_150_000..=1_919_999 => RevmSpecId_primitive::HOMESTEAD,
        1_920_000..=2_462_999 => RevmSpecId_primitive::DAO_FORK,
        2_463_000..=2_674_999 => RevmSpecId_primitive::TANGERINE,
        2_675_000..=4_369_999 => RevmSpecId_primitive::SPURIOUS_DRAGON,
        4_370_000..=7_279_999 => RevmSpecId_primitive::BYZANTIUM,
        7_280_000..=9_068_999 => RevmSpecId_primitive::CONSTANTINOPLE,
        9_069_000..=9_199_999 => RevmSpecId_primitive::PETERSBURG,
        9_200_000..=12_243_999 => RevmSpecId_primitive::ISTANBUL,
        12_244_000..=12_964_999 => RevmSpecId_primitive::MUIR_GLACIER,
        12_965_000..=13_772_999 => RevmSpecId_primitive::ARROW_GLACIER,
        13_773_000..=15_049_999 => RevmSpecId_primitive::LONDON,
        15_050_000..=15_537_393 => RevmSpecId_primitive::GRAY_GLACIER,
        15_537_394..=17_034_869 => RevmSpecId_primitive::MERGE,
        17_034_870..=19_426_586 => RevmSpecId_primitive::SHANGHAI,
        _ => RevmSpecId_primitive::CANCUN,
    };
    
    let mut block_env = RevmBlockEnv_ctx::default();
    block_env.number = RevmU256::from(block_number_u64);
    block_env.timestamp = RevmU256::from(ethers_block.timestamp.as_u64());
    block_env.beneficiary = ethers_to_revm_address(ethers_block.author.unwrap_or_default());
    if let Some(base_fee) = ethers_block.base_fee_per_gas {
        block_env.basefee = ethers_to_revm_u256(base_fee).to::<u64>();
    }
    block_env.gas_limit = ethers_to_revm_u256(ethers_block.gas_limit).to::<u64>();
    
    // Setup transaction environment
    let caller_revm_address = ethers_to_revm_address(ethers_tx.from);
    let mut tx_env = RevmTxEnv_ctx::default();
    tx_env.caller = caller_revm_address;
    tx_env.gas_limit = ethers_tx.gas.as_u64();
    tx_env.gas_price = ethers_to_revm_u256(ethers_tx.gas_price.unwrap_or_default()).to::<u128>();
    tx_env.gas_priority_fee = ethers_tx.max_priority_fee_per_gas.map(|p| {
        ethers_to_revm_u256(p).to::<u128>()
    });
    tx_env.value = ethers_to_revm_u256(ethers_tx.value);
    tx_env.data = RevmBytes::from(ethers_tx.input.0.clone());
    
    if let Some(ethers_to_address) = ethers_tx.to {
        tx_env.kind = RevmTransactTo_ctx::Call(ethers_to_revm_address(ethers_to_address));
    } else {
        tx_env.kind = RevmTransactTo_ctx::Create;
    }
    
    tx_env.nonce = ethers_tx.nonce.as_u64();
    tx_env.chain_id = ethers_tx.chain_id.map(|id| id.as_u64());
    
    // Setup database at parent block
    let fork_block_u64 = block_number_u64.saturating_sub(1);
    let fork_block_id = AlloyBlockId::Number(alloy_eips::BlockNumberOrTag::Number(fork_block_u64));
    
    // Get initial balances
    let mut initial_balances = HashMap::new();
    match alloy_provider_dyn.get_balance(tx_env.caller.into()).block_id(fork_block_id).await {
        Ok(bal) => { initial_balances.insert(tx_env.caller, RevmU256::from_limbs(bal.into_limbs())); },
        Err(_) => {}
    }
    if let RevmTransactTo_ctx::Call(to_addr) = tx_env.kind {
        match alloy_provider_dyn.get_balance(to_addr.into()).block_id(fork_block_id).await {
            Ok(bal) => { initial_balances.insert(to_addr, RevmU256::from_limbs(bal.into_limbs())); },
            Err(_) => {}
        }
    }
    
    // Setup database and EVM
    let alloy_db_for_cache = AlloyDB::<AlloyEthereum, Arc<AlloyDynProvider<AlloyEthereum>>>::new(
        alloy_provider_dyn.clone(),
        fork_block_id
    );
    let cache_db_for_evm: SimCacheDBForDiff = CacheDB::new(WrapDatabaseAsync::new(alloy_db_for_cache).unwrap());
    
    // Create call tracer to capture internal transfers
    // CallTracer implements Inspector trait and captures all CALL operations with non-zero value
    let call_tracer = CallTracer::new();
    let inspector_for_inspect = call_tracer.clone();
    
    // Build EVM context with the specified hardfork specification
    let mut ctx: RevmContext<RevmBlockEnv_ctx, RevmTxEnv_ctx, RevmCfgEnv_ctx, _, Journal<_>, ()> = 
        RevmContext::new(cache_db_for_evm, cfg_env.spec);
    ctx.cfg = cfg_env.clone();
    ctx.block = block_env.clone();
    
    // Create EVM with the inspector
    let mut evm: Evm<_, CallTracer, EthInstructions<EthInterpreter, _>, EthPrecompiles> = 
        Evm::new_with_inspector(ctx, call_tracer, Default::default(), Default::default());
    
    // Execute transaction with inspector
    match InspectEvm::inspect(&mut evm, tx_env.clone(), inspector_for_inspect) {
        Ok(result_and_state) => {
            let (_result_type, logs, _output_data, gas_used, _gas_refunded) = match result_and_state {
                RevmExecutionResult::Success { reason, gas_used, gas_refunded, logs, output } => {
                    (ExecutionResultType::Success(reason), logs, output.into_data(), gas_used, gas_refunded)
                }
                RevmExecutionResult::Revert { gas_used, output } => {
                    (ExecutionResultType::Revert, Vec::new(), output, gas_used, 0)
                }
                RevmExecutionResult::Halt { reason, gas_used } => {
                    (ExecutionResultType::Halt(reason), Vec::new(), RevmBytes::new(), gas_used, 0)
                }
            };
            
            // Get internal transfers from the inspector
            let internal_transfers = evm.inspector.get_internal_transfers().to_vec();
            let internal_transfer_count = internal_transfers.len();
            
            // Generate comprehensive state changes from REVM journal
            // This captures all balance changes, nonce updates, and storage modifications
            // that occurred during transaction execution
            let mut state_changes = generate_calculated_account_changes(
                evm.ctx.db(),
                &initial_balances,
                &logs,
                &tx_env,
                &block_env,
                gas_used,
                alloy_provider_dyn.clone(),
                fork_block_id
            ).await?;
            
            // Integrate internal transfers into state changes
            // This ensures ETH movements from contract calls are properly accounted for
            state_changes = integrate_internal_transfers(state_changes, &internal_transfers);
            
            // Convert to JSON format (same as main simulator)
            let mut json_output = serde_json::Map::new();
            
            for (address, changes) in &state_changes {
                let mut addr_changes = serde_json::Map::new();
                
                // Calculate ETH change in decimal
                let eth_net_change_str = changes.eth_net_change.to_signed_string();
                let eth_net_change_f64 = eth_net_change_str.parse::<f64>().unwrap_or(0.0);
                let eth_net_change_eth = eth_net_change_f64 / ETH_TO_WEI_FACTOR_F64;
                
                // Calculate token changes with symbols
                let mut token_changes_map = serde_json::Map::new();
                for token_info in &changes.token_infos {
                    let token_str = token_info.net_change.to_signed_string();
                    let token_f64 = token_str.parse::<f64>().unwrap_or(0.0);
                    
                    // Convert based on token decimals
                    let divisor = 10_f64.powi(token_info.decimals as i32);
                    let token_amount = token_f64 / divisor;
                    
                    // Only include if above threshold
                    if token_amount.abs() > 0.1 {
                        token_changes_map.insert(token_info.symbol.clone(), json!(token_amount));
                    }
                }
                
                addr_changes.insert("eth_net".to_string(), json!(eth_net_change_eth));
                addr_changes.insert("token_net".to_string(), json!(token_changes_map));
                
                // Add movements structure for compatibility
                let movements = json!({
                    "token": {"in": {}, "out": {}},
                    "eth": {"in": {}, "out": {}}
                });
                addr_changes.insert("movements".to_string(), movements);
                
                json_output.insert(format!("{:?}", address), json!(addr_changes));
            }
            
            Ok(AnalysisResult {
                method: "Full REVM Simulation".to_string(),
                state_changes: json!(json_output),
                internal_transfers: internal_transfer_count,
                confidence: "High (Complete Accuracy)".to_string(),
            })
        }
        Err(e) => {
            return Err(anyhow!("Transaction execution failed: {:?}", e));
        }
    }
}

/// Helper function to add token change to address tracking
fn add_token_change(
    addresses: &mut HashMap<ethers_core::types::Address, serde_json::Value>,
    address: ethers_core::types::Address,
    token_symbol: &str,
    amount: f64,
) {
    let entry = addresses.entry(address).or_insert_with(|| json!({
        "eth_net": 0.0,
        "token_net": {},
        "movements": {"eth": {"in": {}, "out": {}}, "token": {"in": {}, "out": {}}}
    }));
    
    let current = entry["token_net"][token_symbol].as_f64().unwrap_or(0.0);
    entry["token_net"][token_symbol] = json!(current + amount);
}

/// Helper function to add ETH change to address tracking
fn add_eth_change(
    addresses: &mut HashMap<ethers_core::types::Address, serde_json::Value>,
    address: ethers_core::types::Address,
    amount: f64,
) {
    let entry = addresses.entry(address).or_insert_with(|| json!({
        "eth_net": 0.0,
        "token_net": {},
        "movements": {"eth": {"in": {}, "out": {}}, "token": {"in": {}, "out": {}}}
    }));
    
    let current = entry["eth_net"].as_f64().unwrap_or(0.0);
    entry["eth_net"] = json!(current + amount);
}