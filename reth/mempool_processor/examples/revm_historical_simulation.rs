// rust/mempool_processor/examples/revm_historical_simulation.rs

/*
Algorithm Description:
This example demonstrates live mempool transaction simulation using REVM and state diff analysis.
The process involves:
1. Initialize MempoolFetcher to get live transactions from the Ethereum mempool
2. Set up TransactionSimulator with REVM using the local node
3. For each transaction (up to 20):
   a. Fetch the transaction from mempool
   b. Set up BlockEnv context from the latest block
   c. Initialize SimCacheDB forking from the previous block
   d. Convert TransactionView to REVM TxEnv format
   e. Execute simulation using revm_tx_simulator_lib::simulate_transaction
   f. Generate detailed state changes using generate_calculated_account_changes
   g. Log transaction details and state changes to file for Etherscan comparison
4. Output includes ETH balance changes, token transfers, gas fees, and detailed movement tracking
*/

// --- Standard and External Crates ---
use std::collections::HashMap;
use std::sync::Arc;
use std::fs;
use eyre::{anyhow, Result, WrapErr};
use tracing::{info, warn, error, debug};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender;
use hex;

// --- Ethers Imports ---
use ethers::providers::{Http as EthersHttp, Middleware, Provider as EthersProvider};
use ethers::types::{
    BlockId as EthersBlockId, BlockNumber as EthersBlockNumber,
};

// --- REVM Imports ---
use revm_context::{
    BlockEnv as RevmBlockEnv,
    CfgEnv as RevmCfgEnv,
    TxEnv as RevmTxEnv,
    TransactTo as RevmTransactTo,
    transaction::AccessList as RevmAccessList,
};
use revm_primitives::{
    Address as RevmAddress,
    U256 as RevmU256,
    B256 as RevmB256,
    Bytes as RevmBytes,
    hardfork::SpecId,
};
use revm::{
    bytecode::Bytecode,
    Database,
    state::{Account, AccountInfo, AccountStatus},
};

// --- Alloy Imports (for REVM's DB) ---
use alloy_provider::{ProviderBuilder, Provider as AlloyProviderTrait, DynProvider};
use alloy_network::Ethereum as AlloyEthereum;
use alloy_eips::BlockId as AlloyBlockId;
use revm::database::{CacheDB, WrapDatabaseAsync, AlloyDB};

// --- Local Crate Imports ---
use mempool_processor::mempool_processor::{
    fetcher::MempoolFetcher,
    TransactionSource,
};
use mempool_processor::mempool_processor::types::TransactionView;

// --- revm_tx_simulator_lib imports ---
use revm_tx_simulator_lib::simulation_core::{
    simulate_transaction, 
    SimCacheDB, 
    ExecutionResultType,
};
use revm_tx_simulator_lib::state_diff_utils::{generate_calculated_account_changes, CalculatedAccountChanges};
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};

// Configuration
const RPC_URL: &str = "http://localhost:8545";
const CHAIN_ID: u64 = 1; 
const SPEC_ID_TO_USE: SpecId = SpecId::CANCUN;
const LOG_DIR: &str = "/home/nima/code/crypto/logs/mempool";
const MAX_TXS_TO_PROCESS: usize = 20;

// --- Dummy DB for REVM (Not used in this example but keeping for reference) ---
#[derive(Debug, Clone, Default)]
pub struct MemoryDB {
    pub accounts: HashMap<RevmAddress, Account>,
    pub contracts: HashMap<RevmB256, Bytecode>,
    pub block_hashes: HashMap<u64, RevmB256>,
    pub storages: HashMap<RevmAddress, HashMap<RevmU256, RevmU256>>,
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryDBError {
    #[error("Account not found: {0:?}")]
    AccountNotFound(RevmAddress),
    #[error("Block hash not found for block: {0}")]
    BlockHashNotFound(u64),
    #[error("Serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Provider error: {0}")]
    Provider(String), 
    #[error("REVM DB Error: {0}")]
    RevmDbError(String),
}

impl revm_context::DBErrorMarker for MemoryDBError {}

impl revm::DatabaseCommit for MemoryDB {
    fn commit(&mut self, changes: HashMap<RevmAddress, Account>) {
        for (address, account_changes) in changes {
            if account_changes.status == AccountStatus::SelfDestructed {
                 self.accounts.remove(&address);
                 self.storages.remove(&address);
            } else {
                self.accounts.insert(address, account_changes.clone());
                let account_storage_map = self.storages.entry(address).or_default();
                account_storage_map.clear();
                for (key, slot) in account_changes.storage.iter() {
                    account_storage_map.insert(*key, slot.present_value());
                }
            }
        }
    }
}

impl Database for MemoryDB {
    type Error = MemoryDBError;

    fn basic(&mut self, address: RevmAddress) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(self.accounts.get(&address).map(|acc| acc.info.clone()))
    }

    fn code_by_hash(&mut self, code_hash: RevmB256) -> Result<Bytecode, Self::Error> {
        Ok(self.contracts.get(&code_hash).cloned().unwrap_or_else(Bytecode::default))
    }

    fn storage(&mut self, address: RevmAddress, index: RevmU256) -> Result<RevmU256, Self::Error> {
        if let Some(account_storage) = self.storages.get(&address) {
            Ok(account_storage.get(&index).cloned().unwrap_or_default())
        } else {
            Ok(RevmU256::ZERO)
        }
    }

    fn block_hash(&mut self, number: u64) -> Result<RevmB256, Self::Error> {
        self.block_hashes.get(&number).cloned().ok_or_else(|| MemoryDBError::BlockHashNotFound(number))
    }
}

fn transaction_view_to_revm_tx_env(
    tx_view: &TransactionView,
    chain_id_u64: u64,
) -> Result<RevmTxEnv> {
    let transact_to = match &tx_view.to {
        Some(addr_vec) => RevmTransactTo::Call(RevmAddress::from_slice(addr_vec)),
        None => RevmTransactTo::Create, // Corrected: Use enum variant directly
    };

    // TxEnv expects u128 for gas_price, max_fee_per_blob_gas
    // TxEnv expects u64 for nonce
    let gas_price_u128 = tx_view.gas_price.map_or(0u128, |gp| ethers_to_revm_u256(gp).into_limbs()[0] as u128); // Lossy if > u128

    Ok(RevmTxEnv {
        caller: RevmAddress::from_slice(&tx_view.from),
        gas_limit: tx_view.gas_limit.map_or(21000, |gl| gl.as_u64()), 
        gas_price: gas_price_u128, 
        gas_priority_fee: None, 
        kind: transact_to,
        value: ethers_to_revm_u256(tx_view.value),
        data: tx_view.input_data.as_ref().map_or(RevmBytes::new(), |d| RevmBytes::copy_from_slice(d)),
        nonce: tx_view.nonce.map_or(0, |n| n.as_u64()), 
        chain_id: Some(chain_id_u64),
        access_list: RevmAccessList(Vec::new()), 
        blob_hashes: Vec::new(), 
        max_fee_per_blob_gas: 0u128, // Corrected: TxEnv field is u128
        ..Default::default()
    })
}

fn log_transaction_details(tx_view: &TransactionView, tx_index: usize) {
    info!("=== TRANSACTION #{} DETAILS ===", tx_index + 1);
    info!("Hash: 0x{}", hex::encode(&tx_view.hash));
    info!("From: 0x{}", hex::encode(&tx_view.from));
    if let Some(to) = &tx_view.to {
        info!("To: 0x{}", hex::encode(to));
    } else {
        info!("To: CONTRACT_CREATION");
    }
    info!("Value: {} wei", tx_view.value);
    if let Some(gas_limit) = tx_view.gas_limit {
        info!("Gas Limit: {}", gas_limit);
    }
    if let Some(gas_price) = tx_view.gas_price {
        info!("Gas Price: {} wei", gas_price);
    }
    if let Some(nonce) = tx_view.nonce {
        info!("Nonce: {}", nonce);
    }
    if let Some(input_data) = &tx_view.input_data {
        if input_data.len() > 10 {
            info!("Input Data: 0x{}... ({} bytes)", hex::encode(&input_data[..10]), input_data.len());
        } else if !input_data.is_empty() {
            info!("Input Data: 0x{}", hex::encode(input_data));
        } else {
            info!("Input Data: 0x (empty)");
        }
    }
}

fn log_calculated_changes(changes_map: &HashMap<RevmAddress, CalculatedAccountChanges>, tx_index: usize) {
    info!("=== TRANSACTION #{} STATE CHANGES ===", tx_index + 1);
    if changes_map.is_empty() {
        info!("No calculated state changes.");
        return;
    }
    
    for (addr, changes) in changes_map {
        info!("Address: 0x{}", hex::encode(addr.0));
        
        // Convert ETH changes from wei to ETH for readability
        let eth_change_wei = changes.eth_net_change.absolute_value;
        let eth_change_eth = eth_change_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        info!(
            "  ETH Net Change: {}{:.18} ETH ({} wei)",
            if changes.eth_net_change.is_negative { "-" } else { "+" },
            eth_change_eth,
            changes.eth_net_change.to_signed_string()
        );

        if !changes.token_net_changes.is_empty() {
            info!("  Token Net Changes:");
            for (token_addr, net_change) in &changes.token_net_changes {
                info!(
                    "    Token 0x{}: {}",
                    hex::encode(token_addr.0),
                    net_change.to_signed_string()
                );
            }
        }
        
        if !changes.movements.denom.in_list.is_empty() {
            info!("  ETH Inflows:");
            for movement in &changes.movements.denom.in_list {
                let amount_eth = movement.raw_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                info!("    From {}: {:.18} ETH ({} wei)", movement.source_identifier, amount_eth, movement.raw_amount);
            }
        }
        if !changes.movements.denom.out_list.is_empty() {
            info!("  ETH Outflows:");
            for movement in &changes.movements.denom.out_list {
                let amount_eth = movement.raw_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                info!("    To {}: {:.18} ETH ({} wei)", movement.source_identifier, amount_eth, movement.raw_amount);
            }
        }
        if !changes.movements.token.is_empty() {
            info!("  Token Movements:");
            for (token_addr, movements_in_out) in &changes.movements.token {
                if !movements_in_out.in_list.is_empty() {
                    info!("    Token 0x{} Inflows:", hex::encode(token_addr.0));
                    for movement in &movements_in_out.in_list {
                        info!("      From {}: {}", movement.log_identifier, movement.raw_amount);
                    }
                }
                if !movements_in_out.out_list.is_empty() {
                    info!("    Token 0x{} Outflows:", hex::encode(token_addr.0));
                    for movement in &movements_in_out.out_list {
                        info!("      To {}: {}", movement.log_identifier, movement.raw_amount);
                    }
                }
            }
        }
    }
    info!("=== END TRANSACTION #{} STATE CHANGES ===", tx_index + 1);
}

fn analyze_transaction_viability(tx_view: &TransactionView, current_basefee: u64, tx_index: usize) {
    let tx_gas_price = tx_view.gas_price.map_or(0u64, |gp| gp.as_u64());
    
    info!("=== BASEFEE ANALYSIS FOR TRANSACTION #{} ===", tx_index + 1);
    info!("Transaction gas price: {} wei ({:.2} Gwei)", tx_gas_price, tx_gas_price as f64 / 1e9);
    info!("Current block basefee: {} wei ({:.2} Gwei)", current_basefee, current_basefee as f64 / 1e9);
    
    if tx_gas_price < current_basefee {
        let deficit = current_basefee - tx_gas_price;
        let deficit_percent = (deficit as f64 / current_basefee as f64) * 100.0;
        warn!("⚠️  Transaction UNDERPRICED by {} wei ({:.1}%)", deficit, deficit_percent);
        warn!("   This transaction will likely fail in current block but may succeed if:");
        warn!("   - Network congestion decreases (basefee drops)");
        warn!("   - Transaction waits for a future block");
        
        // Calculate scenarios where it would succeed
        let required_basefee_drop = ((current_basefee - tx_gas_price) as f64 / current_basefee as f64) * 100.0;
        info!("   - Basefee needs to drop by {:.1}% to {:.2} Gwei", required_basefee_drop, tx_gas_price as f64 / 1e9);
        
        // Show typical basefee scenarios
        info!("   Basefee scenarios:");
        let scenarios = vec![
            ("Current", current_basefee),
            ("5% drop", current_basefee * 95 / 100),
            ("10% drop", current_basefee * 90 / 100),
            ("15% drop", current_basefee * 85 / 100),
            ("Required", tx_gas_price),
        ];
        
        for (name, basefee) in scenarios {
            let status = if tx_gas_price >= basefee { "✅ PASS" } else { "❌ FAIL" };
            info!("     {}: {} wei ({:.2} Gwei) - {}", name, basefee, basefee as f64 / 1e9, status);
        }
    } else {
        let surplus = tx_gas_price - current_basefee;
        let surplus_percent = (surplus as f64 / current_basefee as f64) * 100.0;
        info!("✅ Transaction VIABLE with surplus of {} wei ({:.1}%)", surplus, surplus_percent);
    }
    info!("=== END BASEFEE ANALYSIS ===");
}

fn analyze_nonce_dependencies(tx_views: &[TransactionView]) {
    info!("=== NONCE DEPENDENCY ANALYSIS ===");
    
    let mut address_nonces: HashMap<Vec<u8>, Vec<(usize, u64)>> = HashMap::new();
    
    // Group transactions by address and collect their nonces
    for (idx, tx) in tx_views.iter().enumerate() {
        if let Some(nonce) = tx.nonce {
            address_nonces
                .entry(tx.from.clone())
                .or_default()
                .push((idx, nonce.as_u64()));
        }
    }
    
    // Sort and analyze nonce chains
    let mut has_dependencies = false;
    for (address, nonce_list) in address_nonces.iter_mut() {
        if nonce_list.len() > 1 {
            has_dependencies = true;
            nonce_list.sort_by_key(|(_, nonce)| *nonce);
            
            info!("Address 0x{} has {} dependent transactions:", hex::encode(address), nonce_list.len());
            for (chain_idx, (tx_idx, nonce)) in nonce_list.iter().enumerate() {
                if chain_idx == 0 {
                    info!("  📍 TX #{}: nonce {} (first in chain)", tx_idx + 1, nonce);
                } else {
                    let prev_nonce = nonce_list[chain_idx - 1].1;
                    let gap = nonce - prev_nonce;
                    if gap == 1 {
                        info!("  🔗 TX #{}: nonce {} (sequential)", tx_idx + 1, nonce);
                    } else {
                        info!("  ⚠️  TX #{}: nonce {} (gap of {} from previous)", tx_idx + 1, nonce, gap - 1);
                    }
                }
            }
        }
    }
    
    if !has_dependencies {
        info!("No nonce dependencies detected in this batch.");
    } else {
        info!("💡 Dependent transactions will fail individually but succeed when executed in sequence.");
    }
    info!("=== END NONCE ANALYSIS ===");
    info!("");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create log directory
    fs::create_dir_all(LOG_DIR)?;
    
    // Set up file logging
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let log_file_name = format!("mempool_simulation_{}.log", timestamp);
    let file_appender = tracing_appender::rolling::never(LOG_DIR, log_file_name.clone());
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    // Set up logging to both console and file
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender).with_ansi(false))
        .with(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    info!("Starting REVM mempool simulation for {} transactions", MAX_TXS_TO_PROCESS);
    info!("Logging to: {}/{}", LOG_DIR, log_file_name);

    let fetcher = MempoolFetcher::new(RPC_URL)
        .wrap_err("Failed to initialize MempoolFetcher")?;
    info!("MempoolFetcher initialized.");

    let mut cfg_env = RevmCfgEnv::default();
    cfg_env.chain_id = CHAIN_ID;
    cfg_env.spec = SPEC_ID_TO_USE.clone();

    let alloy_provider_instance = ProviderBuilder::new()
        .connect(RPC_URL)
        .await
        .wrap_err("Failed to connect to Alloy Provider")?;
    let alloy_provider: Arc<DynProvider<AlloyEthereum>> = Arc::new(alloy_provider_instance.erased());

    info!("Fetching latest block for BlockEnv...");
    let latest_block_ethers = EthersProvider::<EthersHttp>::try_from(RPC_URL)?
        .get_block(EthersBlockId::Number(EthersBlockNumber::Latest))
        .await?
        .ok_or_else(|| anyhow!("Failed to get latest block from Ethers provider"))?;
    
    let block_gas_limit_u64 = latest_block_ethers.gas_limit.as_u64();
    let block_base_fee_u64 = latest_block_ethers.base_fee_per_gas.map_or(0, |bf| bf.as_u64());

    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(latest_block_ethers.number.unwrap_or_default().as_u64().into());
    block_env.beneficiary = latest_block_ethers.author.map_or_else(|| RevmAddress::ZERO, |h160| ethers_to_revm_address(h160));
    block_env.timestamp = ethers_to_revm_u256(latest_block_ethers.timestamp);
    block_env.gas_limit = block_gas_limit_u64;
    block_env.basefee = block_base_fee_u64;
    block_env.difficulty = ethers_to_revm_u256(latest_block_ethers.difficulty);
    block_env.prevrandao = latest_block_ethers.mix_hash.map(|h| RevmB256::from(h.0));
    // blob_excess_gas_and_price is already set to a default value by Default::default()

    info!("BlockEnv configured for block #{:?}", latest_block_ethers.number.unwrap_or_default());
    info!("Current basefee: {} wei ({:.2} Gwei)", block_base_fee_u64, block_base_fee_u64 as f64 / 1e9);

    info!("Fetching transactions from mempool...");
    let tx_views = fetcher.get_transactions().await.wrap_err("Failed to get transactions from mempool")?;

    if tx_views.is_empty() {
        info!("Mempool is empty. No transactions to simulate.");
        return Ok(());
    }

    let txs_to_process = std::cmp::min(tx_views.len(), MAX_TXS_TO_PROCESS);
    info!("Got {} transactions from mempool. Processing {} transactions.", tx_views.len(), txs_to_process);

    analyze_nonce_dependencies(&tx_views);

    for (tx_index, tx_view_to_simulate) in tx_views.iter().take(txs_to_process).enumerate() {
        info!("");
        info!("▶ ▶ ▶ PROCESSING TRANSACTION {} of {} ◀ ◀ ◀", tx_index + 1, txs_to_process);
        
        log_transaction_details(tx_view_to_simulate, tx_index);

        analyze_transaction_viability(tx_view_to_simulate, block_base_fee_u64, tx_index);

        let revm_tx_env = match transaction_view_to_revm_tx_env(tx_view_to_simulate, CHAIN_ID) {
            Ok(env) => env,
            Err(e) => {
                warn!("Failed to convert TransactionView to RevmTxEnv for tx #{}: {}. Skipping.", tx_index + 1, e);
                continue;
            }
        };

        let fork_block_number_u256 = if block_env.number > RevmU256::ZERO {
            block_env.number - RevmU256::from(1)
        } else {
            RevmU256::ZERO 
        };
        let fork_block_u64 = fork_block_number_u256.try_into().unwrap_or(0u64); 
        let fork_block_id_alloy = AlloyBlockId::from(fork_block_u64);
        
        debug!("Setting up REVM SimCacheDB, forking from block number: {}", fork_block_u64);
        let alloy_db_for_revm = AlloyDB::new(alloy_provider.clone(), fork_block_id_alloy);
        let wrapped_db_for_revm = WrapDatabaseAsync::new(alloy_db_for_revm);
        let cache_db_for_revm: SimCacheDB = CacheDB::new(wrapped_db_for_revm.expect("Database wrapping failed unexpectedly"));

        let mut initial_eth_balances_for_diff = HashMap::new();
        let caller_revm_address = revm_tx_env.caller;
        match alloy_provider.get_balance(caller_revm_address.into()).block_id(fork_block_id_alloy).await {
            Ok(bal) => { initial_eth_balances_for_diff.insert(caller_revm_address, bal); },
            Err(e) => warn!("Failed to get initial balance for caller {}: {}", caller_revm_address, e),
        }
        if let RevmTransactTo::Call(to_revm_address) = revm_tx_env.kind {
             match alloy_provider.get_balance(to_revm_address.into()).block_id(fork_block_id_alloy).await {
                Ok(bal) => { initial_eth_balances_for_diff.insert(to_revm_address, bal); },
                Err(e) => warn!("Failed to get initial balance for receiver {}: {}", to_revm_address, e),
            }
        }
        let beneficiary_revm_address = block_env.beneficiary;
        if !initial_eth_balances_for_diff.contains_key(&beneficiary_revm_address) {
            match alloy_provider.get_balance(beneficiary_revm_address.into()).block_id(fork_block_id_alloy).await {
                Ok(bal) => { initial_eth_balances_for_diff.insert(beneficiary_revm_address, bal); },
                Err(e) => warn!("Failed to get initial balance for beneficiary {}: {}", beneficiary_revm_address, e),
            }
        }

        info!("Executing simulation for transaction #{}", tx_index + 1);
        match simulate_transaction(
            revm_tx_env.clone(),
            block_env.clone(),
            cfg_env.clone(),
            cache_db_for_revm, 
        ) {
            Ok((sim_output, final_db_state)) => {
                info!(
                    "Simulation successful for tx #{}: {:?}, Gas Used: {}, Gas Refunded: {}",
                    tx_index + 1, sim_output.result_type, sim_output.gas_used, sim_output.gas_refunded
                );

                if matches!(sim_output.result_type, ExecutionResultType::Success(_)) {
                    match generate_calculated_account_changes(
                        &final_db_state, 
                        &initial_eth_balances_for_diff, 
                        &sim_output.logs,
                        &revm_tx_env, 
                        &block_env,   
                        sim_output.gas_used, 
                        alloy_provider.clone(), 
                        fork_block_id_alloy, 
                    ).await {
                        Ok(changes) => {
                            log_calculated_changes(&changes, tx_index);
                        }
                        Err(e) => {
                            error!("Failed to generate calculated state changes for tx #{}: {}", tx_index + 1, e);
                        }
                    }
                } else {
                    info!("Transaction #{} did not succeed (Reverted or Halted). Attempting to get gas-related changes.", tx_index + 1);
                     match generate_calculated_account_changes(
                        &final_db_state, 
                        &initial_eth_balances_for_diff, 
                        &sim_output.logs,
                        &revm_tx_env, 
                        &block_env,   
                        sim_output.gas_used, 
                        alloy_provider.clone(), 
                        fork_block_id_alloy, 
                    ).await {
                        Ok(changes) => {
                            info!("State changes for non-successful tx #{}:", tx_index + 1);
                            log_calculated_changes(&changes, tx_index);
                        }
                        Err(e) => {
                            error!("Failed to generate calculated state changes for non-successful tx #{}: {}", tx_index + 1, e);
                        }
                    }
                }
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("NonceTooHigh") {
                    warn!("🔗 NONCE DEPENDENCY: Transaction #{} requires previous transactions from same address to be executed first", tx_index + 1);
                    info!("   This transaction will succeed once prerequisite transactions are mined.");
                } else if error_str.contains("GasPriceLessThanBasefee") {
                    warn!("💰 GAS PRICE ISSUE: Transaction #{} gas price too low for current basefee", tx_index + 1);
                    info!("   This transaction may succeed in future blocks if basefee decreases.");
                } else if error_str.contains("InsufficientFunds") {
                    warn!("💸 INSUFFICIENT FUNDS: Transaction #{} sender lacks sufficient balance", tx_index + 1);
                } else {
                    error!("❌ OTHER ERROR: REVM simulation failed for tx #{}: {}", tx_index + 1, e);
                }
            }
        }
        
        info!("✓ Completed processing transaction #{}", tx_index + 1);
    }
    
    info!("");
    info!("🎉 Finished processing {} transactions from mempool", txs_to_process);
    info!("Logs saved to: {}/{}", LOG_DIR, log_file_name);
    
    // Print summary statistics
    info!("");
    info!("=== SIMULATION SUMMARY ANALYSIS ===");
    info!("📊 Typical failure reasons in mempool simulation:");
    info!("   1. ⚠️  NONCE TOO HIGH: Transaction depends on previous tx not yet mined");
    info!("   2. ⚠️  GAS PRICE < BASEFEE: Transaction viable in future blocks with lower basefee");
    info!("   3. ⚠️  REVERT: Smart contract logic fails (legitimate failure)");
    info!("");
    info!("💡 To improve accuracy:");
    info!("   • Use 'pending' block instead of 'latest' for simulation");
    info!("   • Simulate transaction chains in dependency order");
    info!("   • Test multiple basefee scenarios (current, -5%, -10%)");
    info!("   • Compare results with actual on-chain outcomes");
    info!("=== END SUMMARY ===");
    Ok(())
} 