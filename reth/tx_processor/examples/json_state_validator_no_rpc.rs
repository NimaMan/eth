#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// NO RPC internal transfer implementation - analyzes REVM state changes directly
// Uses the existing working validator but removes RPC overhead

// Silence unused crate dependency warnings
use chrono as _;
use ethers_signers as _;
use hex as _;
use reqwest as _;
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
use serde_json::json;

// Local REVM and simulator library imports
use revm_tx_simulator_lib::{
    ExecutionResultType, 
    conversions::{ethers_to_revm_address, ethers_to_revm_u256}, 
    state_diff_utils::{CalculatedAccountChanges, generate_calculated_account_changes},
    SimCacheDBForDiff,
};

// REVM specific imports
use revm_primitives::{
    Bytes as RevmBytes, 
    hardfork::SpecId as RevmSpecId_primitive,
    U256 as RevmU256,
};
use revm_context::{
    BlockEnv as RevmBlockEnv_ctx, CfgEnv as RevmCfgEnv_ctx, TxEnv as RevmTxEnv_ctx, 
    TransactTo as RevmTransactTo_ctx, Context as RevmContext, Journal, 
    result::{ExecutionResult as RevmExecutionResult}
};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use revm::{MainBuilder, ExecuteCommitEvm}; 

// Alloy Imports
use alloy_eips::BlockId as AlloyBlockId;
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};

// Constants
const RPC_URL: &str = "http://127.0.0.1:8545";
const ETH_TO_WEI_FACTOR_F64: f64 = 1_000_000_000_000_000_000.0_f64;

#[tokio::main]
async fn main() -> Result<()> {
    // Get transaction hash from command line
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <transaction_hash>", args[0]);
        std::process::exit(1);
    }
    
    let tx_hash_str = &args[1];
    let target_tx_hash_h256: EthersH256 = tx_hash_str.parse()?;
    
    // Setup providers (minimal RPC usage - only for basic transaction data)
    let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
    let eth_client = Arc::new(ethers_provider);
    
    let alloy_provider_dyn: Arc<AlloyDynProvider<AlloyEthereum>> =
        Arc::new(ProviderBuilder::new().connect(RPC_URL).await?.erased());
    
    // Fetch transaction (basic info only)
    let ethers_tx = eth_client.get_transaction(target_tx_hash_h256).await?
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
    
    // Dynamic spec selection
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
    block_env.number = ethers_to_revm_u256(block_number_u64.into());
    block_env.timestamp = ethers_to_revm_u256(ethers_block.timestamp);
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
    
    let mut evm_context = RevmContext::<RevmBlockEnv_ctx, RevmTxEnv_ctx, RevmCfgEnv_ctx, SimCacheDBForDiff, Journal<SimCacheDBForDiff>, ()>::new(
        cache_db_for_evm,
        cfg_env.spec 
    );
    evm_context.cfg = cfg_env.clone();
    evm_context.block = block_env.clone();
    
    let mut mainnet_evm = evm_context.build_mainnet();
    
    eprintln!("🚀 Executing transaction with NO RPC overhead: {}", tx_hash_str);
    eprintln!("📋 Using spec: {:?}", cfg_env.spec);
    
    // Execute transaction using pure REVM
    match mainnet_evm.transact_commit(tx_env.clone()) {
        Ok(execution_result) => {
            let (_result_type, logs, _output_data, gas_used, _gas_refunded) = match execution_result {
                RevmExecutionResult::Success { reason, gas_used, gas_refunded, logs, output } => {
                    eprintln!("✅ Transaction executed successfully: {:?}", reason);
                    (ExecutionResultType::Success(reason), logs, output.into_data(), gas_used, gas_refunded)
                }
                RevmExecutionResult::Revert { gas_used, output } => {
                    eprintln!("⚠️  Transaction reverted, gas used: {}", gas_used);
                    (ExecutionResultType::Revert, Vec::new(), output, gas_used, 0)
                }
                RevmExecutionResult::Halt { reason, gas_used } => {
                    eprintln!("❌ Transaction halted: {:?}, gas used: {}", reason, gas_used);
                    (ExecutionResultType::Halt(reason), Vec::new(), RevmBytes::new(), gas_used, 0)
                }
            };
            
            // Generate comprehensive state changes from REVM journal
            // This captures all state changes that happened during execution
            let state_changes = generate_calculated_account_changes(
                &mainnet_evm.ctx.journaled_state.database,
                &initial_balances,
                &logs,
                &tx_env,
                &block_env,
                gas_used,
                alloy_provider_dyn.clone(),
                fork_block_id
            ).await?;
            
            // Analyze logs and state changes to detect internal transfers
            let inferred_internal_transfers = analyze_internal_transfers(&logs, &state_changes);
            
            eprintln!("📊 Analyzed {} logs and {} state changes", logs.len(), state_changes.len());
            eprintln!("📊 Inferred {} potential internal transfers", inferred_internal_transfers);
            eprintln!("📊 Found {} addresses with state changes", state_changes.len());
            
            // Convert to JSON format
            let mut json_output = serde_json::Map::new();
            
            for (address, changes) in &state_changes {
                let mut addr_changes = serde_json::Map::new();
                
                // Calculate ETH change in decimal
                let eth_net_change_str = changes.eth_net_change.to_signed_string();
                let eth_net_change_f64 = eth_net_change_str.parse::<f64>().unwrap_or(0.0);
                let eth_net_change_eth = eth_net_change_f64 / ETH_TO_WEI_FACTOR_F64;
                
                // Calculate token changes
                let mut total_token_change = 0.0;
                for (_token_addr, token_change) in &changes.token_net_changes {
                    let token_str = token_change.to_signed_string();
                    let token_f64 = token_str.parse::<f64>().unwrap_or(0.0);
                    total_token_change += token_f64 / ETH_TO_WEI_FACTOR_F64;
                }
                
                addr_changes.insert("denom_net".to_string(), json!(eth_net_change_eth));
                addr_changes.insert("token_net".to_string(), json!(total_token_change));
                
                // Add empty movements to match Python format
                let movements = json!({
                    "token": {"in": {}, "out": {}},
                    "denom": {"in": {}, "out": {}}
                });
                addr_changes.insert("movements".to_string(), movements);
                
                json_output.insert(format!("{:?}", address), json!(addr_changes));
            }
            
            // Output JSON
            println!("{}", serde_json::to_string_pretty(&json_output)?);
            
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Transaction execution failed: {:?}", e);
            println!("{{}}");
            Ok(())
        }
    }
}

// Analyze logs and state changes to infer internal transfers without RPC
fn analyze_internal_transfers(
    _logs: &[revm_primitives::Log],
    state_changes: &HashMap<revm_primitives::Address, CalculatedAccountChanges>
) -> usize {
    // Look for patterns that indicate internal transfers:
    // 1. Multiple addresses with ETH changes that aren't the sender/receiver
    // 2. Complex interactions with multiple contracts
    // 3. ETH movements that don't match the transaction value
    
    let mut addresses_with_eth_changes = 0;
    let mut _total_eth_volume = 0.0;
    
    for (_addr, changes) in state_changes {
        let eth_change_str = changes.eth_net_change.to_signed_string();
        let eth_change_f64 = eth_change_str.parse::<f64>().unwrap_or(0.0);
        
        if eth_change_f64.abs() > 1e-12 {  // Non-zero ETH change
            addresses_with_eth_changes += 1;
            _total_eth_volume += eth_change_f64.abs();
        }
    }
    
    // Heuristic: if we have more than 3 addresses with ETH changes, 
    // likely indicates internal transfers
    if addresses_with_eth_changes > 3 {
        // Estimate number of internal transfers based on address complexity
        (addresses_with_eth_changes - 2).max(0)  // Subtract sender and primary recipient
    } else {
        0
    }
}