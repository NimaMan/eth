#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use anyhow::{anyhow, Result};
use ethers_core::{
    types::{H256 as EthersH256}, // Removed EthersU256_ethers
    // Transaction as EthersTransaction, BlockId as EthersBlockId, BlockNumber as EthersBlockNumber, U64 as EthersU64,
    // Address as EthersAddress, Bytes as EthersBytes,
    // utils::rlp::Decodable as EthersDecodable, 
};
use ethers_providers::{Middleware, Provider as EthersProvider, Http as EthersHttp};
use std::sync::Arc;
use std::fs;
use std::path::Path;
use tracing::{error, info, Level, warn};
// use tracing_subscriber::fmt::format::FmtSpan; // Unused
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;


// Local REVM and simulator library imports
use revm_tx_simulator_lib::{
    SimCacheDB, SimulationOutput, ExecutionResultType, // ExecutionResultType is used in match
    conversions::{ethers_to_revm_address, ethers_to_revm_u256}, 
    // simulate_transaction, // Not used in this example
};

// REVM specific imports
use revm_primitives::{
    Address as RevmAddress, Bytes as RevmBytes, // Log as RevmLog, TxKind as RevmTxKind, // Log is used by SimulationOutput, TxKind maybe later
    B256 as RevmB256, // KECCAK_EMPTY as REVM_KECCAK_EMPTY, U256 as RevmU256, // U256 might be used by SimulationOutput or logging
    hardfork::SpecId as RevmSpecId_primitive, // For SimulationOutput fields if they use it directly or for conversions
    // AccessListItem as RevmAccessListItem, // Removed from here
};
// use revm_state::{AccountInfo as RevmAccountInfo_state, JournaledState}; // JournaledState is used in RevmContext generic
// use revm_state::{Journal}; // Moved to revm_context
// use revm_bytecode::bytecode::Bytecode as RevmBytecode_type; // Unused
use revm_context::{
    BlockEnv as RevmBlockEnv_ctx, CfgEnv as RevmCfgEnv_ctx, TxEnv as RevmTxEnv_ctx, 
    TransactTo as RevmTransactTo_ctx, Context as RevmContext, Journal, // Added Journal from revm_context
    result::ExecutionResult as RevmExecutionResult, // Added from revm_context::result
    transaction::AccessListItem as RevmAccessListItem, // Added here
    ContextTr // Import ContextTr trait for .db() method
};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync}; // Database trait might be unused now
// use revm::database::Database;
use revm::handler::{ExecuteCommitEvm, MainBuilder}; 

// Alloy Imports
// use alloy_primitives::{Address as AlloyAddress, Bytes as AlloyBytes, U256 as AlloyU256}; // Unused directly
use alloy_eips::BlockId as AlloyBlockId;
use alloy_network::Ethereum as AlloyEthereum;
// use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait, RootProvider, Network};
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait}; // Added AlloyProviderTrait for .erased()

// --- Constants ---
// const PRIVKEY_HEX: &str = "59c6995e998f97a5a004498123312aaab5e367d6f4e97d96b2ebe497acf7f5b1"; // Unused
const RPC_URL: &str = "http://127.0.0.1:8545";
const LOG_FILE_PATH_PREFIX: &str = "/home/nima/code/crypto/logs/mempool/";
const NUM_TXS_TO_SIMULATE_FROM_BLOCK: usize = 10; // Simulate up to 10 transactions
// Example Transaction Hash (Uniswap V2 Swap: WETH -> USDC) - Will be replaced by dynamic fetching
// const EXAMPLE_TX_HASH: &str = "0x2c35c2da7871a98a8a85825602ba612c0738835983397d518758d087932f3d15";


// --- Helper to log SimulationOutput (from simulate_various_txs.rs) ---
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

// --- Helper to log state changes (from simulate_various_txs.rs, simplified) ---
fn log_cached_state_summary(tx_label: &str, cache_db: &SimCacheDB) {
    info!("--- {} State Post-Simulation (Summary from CacheDB) ---", tx_label);
    if cache_db.cache.accounts.is_empty() {
        info!("  No accounts found in CacheDB state after transaction.");
    } else {
        info!("  Touched accounts in CacheDB: {}", cache_db.cache.accounts.len());
        for (address, account_state) in cache_db.cache.accounts.iter().take(10) { // Log first 10
            info!("  Account: {:?}, Status: {:?}", address, account_state.account_state);
            info!("    Info: Balance: {}, Nonce: {}, CodeHash: {:?}, Code Loaded: {}",
                account_state.info.balance,
                account_state.info.nonce,
                account_state.info.code_hash,
                account_state.info.code.is_some()
            );
            if account_state.storage.len() > 0 {
                 info!("    Storage (Cached after Tx): {} slots modified/loaded", account_state.storage.len());
            }
        }
        if cache_db.cache.accounts.len() > 10 { info!(" ... and more accounts touched.");}
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let log_file_name = format!("simulate_mempool_tx_{}.log", timestamp);
    let log_dir = Path::new(LOG_FILE_PATH_PREFIX);
    fs::create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::never(log_dir, log_file_name);
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    let log_level_filter = Level::INFO; // Changed from tracing::level_filters::LevelFilter
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender).with_ansi(false))
        .with(tracing_subscriber::filter::Targets::new().with_target(env!("CARGO_CRATE_NAME"), log_level_filter))
        .init();

    info!("Starting simulation of a real mempool/recent transaction.");

    // 1. Setup Ethers Provider (for tx fetching) and Alloy Provider (for AlloyDB)
    let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
    let eth_client = Arc::new(ethers_provider);

    // Alloy Provider Setup
    // let reqwest_http_client = ReqwestClient::new(); // Unused if connect() takes RPC_URL directly
    // let alloy_actual_transport = AlloyHttp::with_client(reqwest_http_client, RPC_URL.parse()?); // Unused
    // let _alloy_rpc_client_obj = AlloyRpcClient::new(alloy_actual_transport, true); // Unused variable

    let alloy_provider_dyn: Arc<AlloyDynProvider<AlloyEthereum>> =
        Arc::new(ProviderBuilder::new().connect(RPC_URL).await?.erased());

    info!("Successfully connected to RPC: {}", RPC_URL);

    // 2. Fetch a recent transaction dynamically
    info!("Fetching latest block to find transactions...");
    let latest_block_ethers = eth_client.get_block_with_txs(ethers_core::types::BlockNumber::Latest).await?
        .ok_or_else(|| anyhow!("Failed to get latest block with transactions"))?;
    
    info!("Found block #{} with {} transactions.", 
        latest_block_ethers.number.unwrap_or_default(), 
        latest_block_ethers.transactions.len());

    // Common CfgEnv and BlockEnv for all transactions in this block
    let chain_id_u64 = eth_client.get_chainid().await?.as_u64();
    let mut cfg_env = RevmCfgEnv_ctx::default();
    cfg_env.chain_id = chain_id_u64;
    cfg_env.spec = RevmSpecId_primitive::SHANGHAI;

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

    // Use the block *before* the transaction's block for the fork state
    let fork_block_number = block_number_u64.saturating_sub(1);
    let fork_block_id = AlloyBlockId::from(fork_block_number);
    info!("Initializing AlloyDB forking from block: {:?}", fork_block_id);

    for (tx_idx, ethers_tx_summary) in latest_block_ethers.transactions.iter().take(NUM_TXS_TO_SIMULATE_FROM_BLOCK).enumerate() {
        let target_tx_hash_h256 = ethers_tx_summary.hash;
        info!("--- Starting simulation for TX #{} from block (Hash: {:?}) ---", tx_idx + 1, target_tx_hash_h256);

        // Fetch full transaction details for TxEnv
        let ethers_tx_opt = eth_client.get_transaction(target_tx_hash_h256).await?;
        let ethers_tx = match ethers_tx_opt {
            Some(tx) => {
                info!("Successfully fetched full transaction details for {:?}", target_tx_hash_h256);
                tx
            }
            None => {
                error!("Transaction {:?} (from block summary) not found when fetching full details. Skipping.", target_tx_hash_h256);
                continue; // Skip to next transaction
            }
        };

        let caller_ethers_address = ethers_tx.from;
        let caller_revm_address = ethers_to_revm_address(caller_ethers_address);

        // 4. Populate TxEnv from fetched EthersTransaction
        let mut tx_env = RevmTxEnv_ctx::default();
        tx_env.caller = caller_revm_address;
        tx_env.gas_limit = ethers_tx.gas.as_u64();
        tx_env.gas_price = ethers_to_revm_u256(ethers_tx.gas_price.unwrap_or_default()).to::<u128>();
        tx_env.gas_priority_fee = ethers_tx.max_priority_fee_per_gas.map(|p| {
            ethers_to_revm_u256(p).to::<u128>()
        });
        tx_env.kind = match ethers_tx.to {
            Some(addr) => RevmTransactTo_ctx::Call(ethers_to_revm_address(addr)),
            None => RevmTransactTo_ctx::Create,
        };
        tx_env.value = ethers_to_revm_u256(ethers_tx.value);
        tx_env.data = RevmBytes(ethers_tx.input.0.clone());
        tx_env.nonce = ethers_tx.nonce.as_u64();
        tx_env.chain_id = ethers_tx.chain_id.map(|id| id.as_u64());
        // Populate access list if present
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

        // 5. Initialize AlloyDB & CacheDB for this specific transaction simulation
        // It's crucial to re-initialize the DB for each tx to ensure independent simulation against the common fork_block_id state.
        let alloy_db_inner_loop = AlloyDB::<AlloyEthereum, Arc<AlloyDynProvider<AlloyEthereum>>>::new(alloy_provider_dyn.clone(), fork_block_id);
        let cache_db_loop: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db_inner_loop).unwrap());
        info!("Re-initialized AlloyDB + CacheDB for tx {:?}", target_tx_hash_h256);
        
        let mut evm_context = RevmContext::<RevmBlockEnv_ctx, RevmTxEnv_ctx, RevmCfgEnv_ctx, SimCacheDB, Journal<SimCacheDB>, ()>::new(
            cache_db_loop, // Use the per-transaction DB
            cfg_env.spec 
        );
        evm_context.cfg = cfg_env.clone(); // Clone CfgEnv if it could be modified per tx (safer)
        evm_context.block = block_env.clone(); // Clone BlockEnv (safer, though likely static for the block)

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
                    logs,
                    output_data,
                };

                log_simulation_output(&format!("TX {:?}", target_tx_hash_h256), &sim_output);
                log_cached_state_summary(&format!("TX {:?}", target_tx_hash_h256), mainnet_evm.ctx.db());
            }
            Err(e) => {
                error!("REVM Execution Failed for TX {:?}: {:?}", target_tx_hash_h256, e);
            }
        }
        info!("--- Finished simulation for TX #{} (Hash: {:?}) ---", tx_idx + 1, target_tx_hash_h256);
    }
    
    info!("Finished simulation of up to {} transactions from the latest block.", NUM_TXS_TO_SIMULATE_FROM_BLOCK);
    Ok(())
} 