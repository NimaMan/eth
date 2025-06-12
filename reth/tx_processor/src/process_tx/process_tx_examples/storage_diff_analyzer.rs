#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// Silence unused crate dependency warnings
use clap as _;
use ethers_signers as _;
use eyre as _;
use reqwest as _;
use reth_ethereum as _;
use revm_database as _;
use revm_handler as _;
use revm_inspector as _;
use revm_interpreter as _;
use revm_state as _;
use serde as _;
use serde_json as _;

use anyhow::{anyhow, Result};
use ethers_core::types::H256 as EthersH256;
use ethers_providers::{Middleware, Provider as EthersProvider, Http as EthersHttp};
use std::sync::Arc;
use std::fs;
use std::path::Path;
use tracing::{error, info, Level, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use std::collections::HashMap;

// Local REVM and simulator library imports
use revm_tx_simulator_lib::{
    SimulationOutput, ExecutionResultType, 
    conversions::{ethers_to_revm_address, ethers_to_revm_u256}, 
    state_diff_utils::{CalculatedAccountChanges, generate_calculated_account_changes},
    SimCacheDBForDiff, 
    spec_id_from_block_number,
};

// REVM specific imports
use revm_primitives::{
    Address as RevmAddress, Bytes as RevmBytes, 
    B256 as RevmB256, 
    hardfork::SpecId as RevmSpecId_primitive,
    Log as RevmLog, 
    U256 as RevmU256,
};
use revm_context::{
    BlockEnv as RevmBlockEnv_ctx, CfgEnv as RevmCfgEnv_ctx, TxEnv as RevmTxEnv_ctx, 
    TransactTo as RevmTransactTo_ctx, Context as RevmContext, Journal, 
    result::{ExecutionResult as RevmExecutionResult}, // Removed RevmOutput
    transaction::AccessListItem as RevmAccessListItem, 
    ContextTr 
};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync}; 
use revm::handler::{ExecuteCommitEvm, MainBuilder}; 

// Alloy Imports
use alloy_eips::BlockId as AlloyBlockId;
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};

// --- Constants ---
const RPC_URL: &str = "http://127.0.0.1:8545";
const LOG_FILE_PATH_PREFIX: &str = "/home/nima/code/crypto/logs/mempool/";
const NUM_TXS_TO_SIMULATE_FROM_BLOCK: usize = 10;
const ERC20_TRANSFER_EVENT_SIGNATURE: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const ETH_TO_WEI_FACTOR_F64: f64 = 1_000_000_000_000_000_000.0_f64;

// --- Helper to log SimulationOutput ---
fn log_simulation_output(tx_label: &str, sim_output: &SimulationOutput) {
    info!("--- {} Simulation Result ---", tx_label);
    info!("Result Type: {:?}", sim_output.result_type);
    info!("Gas Used: {}", sim_output.gas_used);
    info!("Gas Refunded: {}", sim_output.gas_refunded);
    info!("Output Data: 0x{}", hex::encode(&sim_output.output_data));
    info!("Event Logs Emitted: {}", sim_output.logs.len());
    for (log_idx, log_entry) in sim_output.logs.iter().enumerate() {
        info!("  Log #{}: Address: {:?}, Topics: {:?}, Data: 0x{}",
            log_idx + 1,
            log_entry.address,
            log_entry.topics(),
            hex::encode(&log_entry.data.data)
        );
    }
}

// --- NEW Helper to log CalculatedAccountChanges ---
fn log_calculated_account_changes(tx_label: &str, all_changes: &HashMap<RevmAddress, CalculatedAccountChanges>) {
    info!("--- {} Calculated Account State Changes (Python-like) ---", tx_label);
    if all_changes.is_empty() {
        info!("  No account changes reported.");
        return;
    }

    for (address, changes) in all_changes {
        info!("  Account: {:?}", address);
        
        // Convert eth_net_change to f64 and then to ETH
        let eth_net_change_f64 = changes.eth_net_change.to_signed_string().parse::<f64>().unwrap_or(0.0);
        let eth_net_change_eth = eth_net_change_f64 / ETH_TO_WEI_FACTOR_F64;
        info!("    eth_net_change: {:.18} ETH", eth_net_change_eth);

        if !changes.token_net_changes.is_empty() {
            info!("    token_net_changes:");
            for (token_addr, net_change) in &changes.token_net_changes {
                // Assuming token amounts might also be large, but their decimal places vary.
                // For now, displaying them as raw strings. If specific decimal conversions are needed,
                // that would require token-specific metadata.
                info!("      Token {:?}: {}", token_addr, net_change.to_signed_string());
            }
        }

        if !changes.movements.eth.in_list.is_empty() || !changes.movements.eth.out_list.is_empty() {
            info!("    eth_movements:");
            for movement in &changes.movements.eth.in_list {
                let amount_eth = movement.raw_amount.to_string().parse::<f64>().unwrap_or(0.0) / ETH_TO_WEI_FACTOR_F64;
                info!("      IN:  Source: {}, Amount: {:.18} ETH", movement.source_identifier, amount_eth);
            }
            for movement in &changes.movements.eth.out_list {
                let amount_eth = movement.raw_amount.to_string().parse::<f64>().unwrap_or(0.0) / ETH_TO_WEI_FACTOR_F64;
                info!("      OUT: Source: {}, Amount: {:.18} ETH", movement.source_identifier, amount_eth);
            }
        }

        if !changes.movements.token.is_empty() {
            info!("    token_movements:");
            for (token_addr, token_movements_in_out) in &changes.movements.token {
                info!("      Token Contract: {:?}", token_addr);
                for movement in &token_movements_in_out.in_list {
                    info!("        IN:  {}, Amount: {}", movement.log_identifier, movement.raw_amount);
                }
                for movement in &token_movements_in_out.out_list {
                    info!("        OUT: {}, Amount: {}", movement.log_identifier, movement.raw_amount);
                }
            }
        }
    }
}

// --- Helper to log ERC20 Transfers ---
fn log_erc20_transfers(tx_label: &str, logs: &[RevmLog]) {
    info!("--- {} ERC20 Transfers Detected ---", tx_label);
    let mut count = 0;
    for log_entry in logs {
        if log_entry.topics().len() == 3 && 
           hex::encode(log_entry.topics()[0].as_slice()) == ERC20_TRANSFER_EVENT_SIGNATURE {
            count += 1;
            let token_contract = log_entry.address;
            let from_address_bytes: [u8; 20] = log_entry.topics()[1].as_slice()[12..].try_into().unwrap_or_default();
            let to_address_bytes: [u8; 20] = log_entry.topics()[2].as_slice()[12..].try_into().unwrap_or_default();
            let from_address = RevmAddress::from(from_address_bytes);
            let to_address = RevmAddress::from(to_address_bytes);
            
            let mut amount_bytes = [0u8; 32];
            let data_slice = log_entry.data.data.as_ref();
            let data_len = data_slice.len();
            if data_len > 0 && data_len <= 32 {
                let start_index = 32 - data_len;
                amount_bytes[start_index..].copy_from_slice(data_slice);
            } else if data_len > 32 {
                warn!("ERC20 Transfer event data for amount is unexpectedly long ({} bytes) in {}. Setting amount to 0.", data_len, tx_label);
            }
            let amount = RevmU256::from_be_bytes(amount_bytes);

            info!("  #{}: Contract: {:?}, From: {:?}, To: {:?}, Amount: {}", 
                count, token_contract, from_address, to_address, amount);
        }
    }
    if count == 0 {
        info!("  No standard ERC20 Transfer events found.");
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let log_file_name = format!("extract_diffs_{}.log", timestamp);
    let log_dir = Path::new(LOG_FILE_PATH_PREFIX);
    fs::create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::never(log_dir, log_file_name);
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    let log_level_filter = Level::INFO; 
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender).with_ansi(false))
        .with(tracing_subscriber::filter::Targets::new().with_target(env!("CARGO_CRATE_NAME"), log_level_filter))
        .init();

    info!("Starting simulation of recent transactions and extracting state diffs.");

    let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
    let eth_client = Arc::new(ethers_provider);

    let alloy_provider_dyn: Arc<AlloyDynProvider<AlloyEthereum>> =
        Arc::new(ProviderBuilder::new().connect(RPC_URL).await?.erased());

    info!("Successfully connected to RPC: {}", RPC_URL);

    info!("Fetching latest block to find transactions...");
    let latest_block_ethers = eth_client.get_block_with_txs(ethers_core::types::BlockNumber::Latest).await?
        .ok_or_else(|| anyhow!("Failed to get latest block with transactions"))?;
    
    info!("Found block #{} with {} transactions.", 
        latest_block_ethers.number.unwrap_or_default(), 
        latest_block_ethers.transactions.len());

    let chain_id_u64 = eth_client.get_chainid().await?.as_u64();
    let mut cfg_env = RevmCfgEnv_ctx::default();
    cfg_env.chain_id = chain_id_u64;
    cfg_env.spec = spec_id_from_block_number(block_number_u64);

    let block_number_u64 = latest_block_ethers.number.unwrap_or_default().as_u64();
    let mut block_env = RevmBlockEnv_ctx::default();
    block_env.number = ethers_to_revm_u256(block_number_u64.into());
    block_env.timestamp = ethers_to_revm_u256(latest_block_ethers.timestamp);
    block_env.beneficiary = latest_block_ethers.author.map_or(RevmAddress::ZERO, ethers_to_revm_address);
    block_env.difficulty = ethers_to_revm_u256(latest_block_ethers.difficulty);
    block_env.prevrandao = latest_block_ethers.mix_hash.map(|h: EthersH256| RevmB256::from_slice(h.as_bytes()));
    block_env.basefee = ethers_to_revm_u256(latest_block_ethers.base_fee_per_gas.unwrap_or_default()).to::<u64>();
    block_env.gas_limit = ethers_to_revm_u256(latest_block_ethers.gas_limit).to::<u64>();

    info!("Setup CfgEnv: Chain ID {}, Spec ID {:?}", cfg_env.chain_id, cfg_env.spec);
    info!("Setup BlockEnv for block {}: Timestamp {}, BaseFee {}", block_env.number, block_env.timestamp, block_env.basefee);

    let fork_block_number = block_number_u64.saturating_sub(1);
    let fork_block_id = AlloyBlockId::from(fork_block_number);
    info!("Initializing AlloyDB forking from block: {:?}", fork_block_id);

    for (tx_idx, ethers_tx_summary) in latest_block_ethers.transactions.iter().take(NUM_TXS_TO_SIMULATE_FROM_BLOCK).enumerate() {
        let target_tx_hash_h256 = ethers_tx_summary.hash;
        info!("--- Starting simulation for TX #{} from block (Hash: {:?}) ---", tx_idx + 1, target_tx_hash_h256);

        let ethers_tx_opt = eth_client.get_transaction(target_tx_hash_h256).await?;
        let ethers_tx = match ethers_tx_opt {
            Some(tx) => {
                info!("Successfully fetched full transaction details for {:?}", target_tx_hash_h256);
                tx
            }
            None => {
                error!("Transaction {:?} (from block summary) not found when fetching full details. Skipping.", target_tx_hash_h256);
                continue; 
            }
        };

        let caller_ethers_address = ethers_tx.from;
        let caller_revm_address = ethers_to_revm_address(caller_ethers_address);

        let mut tx_env = RevmTxEnv_ctx::default();
        tx_env.caller = caller_revm_address;
        tx_env.gas_limit = ethers_tx.gas.as_u64();
        tx_env.gas_price = ethers_to_revm_u256(ethers_tx.gas_price.unwrap_or_default()).to::<u128>();
        tx_env.gas_priority_fee = ethers_tx.max_priority_fee_per_gas.map(|p| {
            ethers_to_revm_u256(p).to::<u128>()
        });
        let to_revm_address_opt = match ethers_tx.to {
            Some(addr) => Some(ethers_to_revm_address(addr)),
            None => None,
        };
        tx_env.kind = match to_revm_address_opt {
            Some(addr) => RevmTransactTo_ctx::Call(addr),
            None => RevmTransactTo_ctx::Create,
        };
        tx_env.value = ethers_to_revm_u256(ethers_tx.value);
        tx_env.data = RevmBytes(ethers_tx.input.0.clone());
        tx_env.nonce = ethers_tx.nonce.as_u64();
        tx_env.chain_id = ethers_tx.chain_id.map(|id| id.as_u64());
        if let Some(access_list_ethers) = &ethers_tx.access_list {
            let collected_list: Vec<RevmAccessListItem> = access_list_ethers.0.iter().map(|item| {
                RevmAccessListItem {
                    address: ethers_to_revm_address(item.address),
                    storage_keys: item.storage_keys.iter().map(|key| RevmB256::from_slice(key.as_bytes())).collect(),
                }
            }).collect();
            tx_env.access_list = revm_context::transaction::AccessList(collected_list);
        }

        info!("Populated TxEnv for tx {:?}: Caller {}, To {:?}, Nonce {:?}, Value {}", 
            target_tx_hash_h256, tx_env.caller, tx_env.kind, tx_env.nonce, tx_env.value);

        let mut initial_balances = HashMap::new();
        match alloy_provider_dyn.get_balance(tx_env.caller.into()).block_id(fork_block_id).await {
            Ok(bal) => { initial_balances.insert(tx_env.caller, RevmU256::from_limbs(bal.into_limbs())); },
            Err(e) => { warn!("Failed to get initial balance for caller {}: {}", tx_env.caller, e); }
        }
        if let RevmTransactTo_ctx::Call(to_addr) = tx_env.kind {
            match alloy_provider_dyn.get_balance(to_addr.into()).block_id(fork_block_id).await {
                Ok(bal) => { initial_balances.insert(to_addr, RevmU256::from_limbs(bal.into_limbs())); },
                Err(e) => { warn!("Failed to get initial balance for recipient {}: {}", to_addr, e); }
            }
        }

        let alloy_db_for_cache = AlloyDB::<AlloyEthereum, Arc<AlloyDynProvider<AlloyEthereum>>>::new(
            alloy_provider_dyn.clone(),
            fork_block_id
        );
        let cache_db_for_evm: SimCacheDBForDiff = CacheDB::new(WrapDatabaseAsync::new(alloy_db_for_cache).unwrap());
        info!("Re-initialized AlloyDB + CacheDB for EVM execution for tx {:?}", target_tx_hash_h256);
        
        let mut evm_context = RevmContext::<RevmBlockEnv_ctx, RevmTxEnv_ctx, RevmCfgEnv_ctx, SimCacheDBForDiff, Journal<SimCacheDBForDiff>, ()>::new(
            cache_db_for_evm,
            cfg_env.spec 
        );
        evm_context.cfg = cfg_env.clone();
        evm_context.block = block_env.clone();

        let mut mainnet_evm = evm_context.build_mainnet();
        
        match mainnet_evm.transact_commit(tx_env.clone()) { 
            Ok(execution_result) => { 
                let (result_type, logs, output_data, gas_used, gas_refunded) = match execution_result {
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

                let sim_output = SimulationOutput { 
                    result_type,
                    gas_used,
                    gas_refunded,
                    logs: logs.clone(), 
                    output_data,
                };

                let tx_label_str = format!("TX {:?}", target_tx_hash_h256);
                log_simulation_output(&tx_label_str, &sim_output);
                log_erc20_transfers(&tx_label_str, &sim_output.logs); 

                info!("Attempting to extract final touched account states for {}...", tx_label_str);
                match generate_calculated_account_changes(
                    mainnet_evm.ctx.db(), 
                    &initial_balances, 
                    &sim_output.logs, 
                    &tx_env, 
                    &block_env, 
                    sim_output.gas_used,
                    alloy_provider_dyn.clone(),
                    fork_block_id
                ).await {
                    Ok(calculated_changes) => {
                        log_calculated_account_changes(&tx_label_str, &calculated_changes);
                    }
                    Err(e) => {
                        error!("Failed to generate calculated account changes for {}: {}", tx_label_str, e);
                    }
                }
            }
            Err(e) => {
                error!("REVM Execution Failed for TX {:?}: {:?}", target_tx_hash_h256, e);
            }
        }
        info!("--- Finished simulation and simplified extraction for TX #{} (Hash: {:?}) ---", tx_idx + 1, target_tx_hash_h256);
    }
    
    info!("Finished simulation and simplified extraction for up to {} transactions from the latest block.", NUM_TXS_TO_SIMULATE_FROM_BLOCK);
    Ok(())
} 