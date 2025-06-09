#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// Silence unused crate dependency warnings
use chrono as _;
use clap as _;
use eyre as _;
use reqwest as _;
use reth_ethereum as _;
use revm_database as _;
use revm_handler as _;
use revm_inspector as _;
use revm_interpreter as _;
use serde as _;
use serde_json as _;

use anyhow::{anyhow, Result};
use ethers_core::{
    types::{
        transaction::eip2718::TypedTransaction, Address as EthersAddress, Bytes as EthersBytes,
        NameOrAddress, Transaction as EthersTransaction, TransactionRequest, U256 as EthersU256,
        BlockId as EthersBlockId, // Renamed to avoid conflict with alloy_eips::BlockId
        BlockNumber as EthersBlockNumber, // Renamed
        H256 as EthersH256, // Keep for mix_hash if needed
        U64 as EthersU64, // For chain_id in TransactionRequest and block numbers
    },
    utils::rlp::Decodable as EthersDecodable,
};
// Ethers provider for our client
use ethers_providers::{Middleware, Provider as EthersProvider, Http};
use ethers_signers::{LocalWallet, Signer};

// Alloy provider for AlloyDB
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};
use alloy_eips::BlockId as AlloyBlockId; // Used by AlloyDB
use alloy_network::Ethereum as AlloyEthereum;

// Local REVM and simulator library imports
use revm_tx_simulator_lib::*;

// REVM database components for AlloyDB
use revm::database::{AlloyDB, CacheDB};
use revm::database_interface::WrapDatabaseAsync;

// Reverting to direct import from revm_primitives root for AccountInfo and KECCAK_EMPTY
use revm_primitives::{
    Address as RevmAddress, Bytes as RevmBytes, TxKind as RevmTxKind,
    B256 as RevmB256, KECCAK_EMPTY as REVM_KECCAK_EMPTY,
    hardfork::SpecId as RevmSpecId,
};
use revm_state::AccountInfo as RevmAccountInfo; // AccountInfo imported from revm_state
use revm_context::{BlockEnv, CfgEnv, Context as RevmContext, Journal, TxEnv, ContextTr}; // Added ContextTr
use revm_context::result::ExecutionResult; // Import ExecutionResult
use revm::handler::{ExecuteCommitEvm, MainBuilder};

// std
use std::str::FromStr;
use std::sync::Arc;
use std::fs::create_dir_all;
use std::path::Path;

// Tracing
use tracing::{error, info, level_filters::LevelFilter}; // Added LevelFilter

// Type alias for the CacheDB using AlloyDB (similar to revm example)
type RevmAlloyCacheDB = CacheDB<WrapDatabaseAsync<AlloyDB<AlloyEthereum, Arc<AlloyDynProvider>>>>;


// Example constants
// WARNING: This is a well-known test private key for development only - NEVER use in production
const PRIVKEY_HEX: &str = "59c6995e998f97a5a004498123312aaab5e367d6f4e97d96b2ebe497acf7f5b1";
const RPC_URL: &str = "http://127.0.0.1:8545"; // Your local Reth node
const NUM_TX_TO_SIMULATE: u64 = 2; // Changed to 2 for Approve + Swap
const LOG_FILE_PATH: &str = "/home/nima/code/crypto/logs/mempool/mempool_tx_simulation.log";

// Contract Addresses for USDC -> WETH Swap
const USDC_ADDRESS_HEX: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const WETH_ADDRESS_HEX: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const UNISWAP_V2_ROUTER_HEX: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

// Calldata builder for ERC20 approve(address spender, uint256 amount)
// selector: 0x095ea7b3
fn build_approve_calldata(spender_address: EthersAddress, amount: EthersU256) -> EthersBytes {
    let mut data = hex::decode("095ea7b3").unwrap(); // function selector
    let mut word = [0u8; 32];

    // spender_address (padded to 32 bytes)
    word[12..].copy_from_slice(spender_address.as_bytes());
    data.extend_from_slice(&word);
    word.fill(0);

    // amount
    amount.to_big_endian(&mut word);
    data.extend_from_slice(&word);

    EthersBytes::from(data)
}

// Calldata builder for Uniswap V2 swapExactTokensForTokens
// selector: 0x38ed1739
// swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
fn build_swap_calldata(
    amount_in: EthersU256,
    amount_out_min: EthersU256,
    path: Vec<EthersAddress>,
    to_address: EthersAddress,
    deadline: EthersU256,
) -> EthersBytes {
    let mut data = hex::decode("38ed1739").unwrap(); // function selector
    let mut word = [0u8; 32];

    // amountIn
    amount_in.to_big_endian(&mut word);
    data.extend_from_slice(&word);
    word.fill(0);

    // amountOutMin
    amount_out_min.to_big_endian(&mut word);
    data.extend_from_slice(&word);
    word.fill(0);

    // path (offset to path data) - dynamic type
    // Offset will be 5 * 32 = 160 (0xa0) (since there are 5 static parameters before path)
    EthersU256::from(160).to_big_endian(&mut word);
    data.extend_from_slice(&word);
    word.fill(0);

    // to_address (padded to 32 bytes)
    word[12..].copy_from_slice(to_address.as_bytes());
    data.extend_from_slice(&word);
    word.fill(0);

    // deadline
    deadline.to_big_endian(&mut word);
    data.extend_from_slice(&word);
    word.fill(0);

    // --- Dynamic part: path array ---
    // Length of the path array
    EthersU256::from(path.len()).to_big_endian(&mut word);
    data.extend_from_slice(&word);
    word.fill(0);

    // Elements of the path array
    for token_address in path {
        word[12..].copy_from_slice(token_address.as_bytes());
        data.extend_from_slice(&word);
        word.fill(0);
    }

    EthersBytes::from(data)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging to file
    let log_dir = Path::new(LOG_FILE_PATH).parent().unwrap();
    create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "mempool_tx_simulation.log");
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking_appender)
        .with_max_level(LevelFilter::INFO) // Set max log level
        .with_ansi(false) // Disable ANSI color codes for cleaner file logs
        .init();

    info!("Starting mempool-like transaction simulation for {} transactions.", NUM_TX_TO_SIMULATE);

    // 0. Setup Ethers Provider (for tx signing and general RPC) and Alloy Provider (for AlloyDB)
    let ethers_provider = EthersProvider::<Http>::try_from(RPC_URL)?;
    let client = Arc::new(ethers_provider.clone()); // Arc for ethers_provider Middleware usage

    // ProviderBuilder::connect expects &str directly
    let alloy_provider_dyn: Arc<AlloyDynProvider> =
        Arc::new(ProviderBuilder::new().connect(RPC_URL).await?.erased());

    info!("Successfully connected to RPC.");

    // 1. Test credentials & addresses
    let wallet: LocalWallet = PRIVKEY_HEX.parse::<LocalWallet>()?.with_chain_id(1u64);
    let caller_ethers_address = wallet.address();
    let caller_revm_address = ethers_to_revm_address(caller_ethers_address);

    // Parse new contract addresses
    let usdc_ethers_address = EthersAddress::from_str(USDC_ADDRESS_HEX)?;
    let weth_ethers_address = EthersAddress::from_str(WETH_ADDRESS_HEX)?;
    let uniswap_router_ethers_address = EthersAddress::from_str(UNISWAP_V2_ROUTER_HEX)?;

    info!("Using Caller Address (Ethers): {:?}, RevM: {:?}", caller_ethers_address, caller_revm_address);
    info!("USDC Address (Ethers): {:?}", usdc_ethers_address);
    info!("WETH Address (Ethers): {:?}", weth_ethers_address);
    info!("Uniswap V2 Router Address (Ethers): {:?}", uniswap_router_ethers_address);

    let chain_id_ethers_u256 = client.get_chainid().await?;
    let chain_id_u64 = chain_id_ethers_u256.as_u64();
    info!("Fetched Chain ID: {}", chain_id_u64);

    let initial_on_chain_nonce_ethers_u256 = client.get_transaction_count(caller_ethers_address, Some(EthersBlockId::Number(EthersBlockNumber::Latest))).await?;
    info!("Initial (on-chain) nonce for {}: {}", caller_ethers_address, initial_on_chain_nonce_ethers_u256);

    // Main simulation loop
    for i in 0..NUM_TX_TO_SIMULATE {
        let current_loop_nonce = initial_on_chain_nonce_ethers_u256 + EthersU256::from(i);
        info!("--- Simulating Transaction {}/{} (Nonce: {}) ---", i + 1, NUM_TX_TO_SIMULATE, current_loop_nonce);

        // Initialize AlloyDB & CacheDB for each transaction.
        // Clone the Arc'd provider for each new AlloyDB instance.
        let alloy_db_inner = AlloyDB::new(alloy_provider_dyn.clone(), AlloyBlockId::latest());
        let wrapped_alloy_db = WrapDatabaseAsync::new(alloy_db_inner)
            .ok_or_else(|| anyhow!("Failed to wrap AlloyDB with WrapDatabaseAsync for tx {}", i + 1))?;
        let mut cache_db: RevmAlloyCacheDB = CacheDB::new(wrapped_alloy_db); // cache_db is mutable here and used
        info!("Initialized AlloyDB + CacheDB for tx {}.", i + 1);

        // Override balance and nonce for the caller in the CacheDB
        let initial_balance_revm = ethers_to_revm_u256(EthersU256::from(10).pow(EthersU256::from(18)) * EthersU256::from(100)); // 100 ETH
        let account_info = RevmAccountInfo {
            balance: initial_balance_revm,
            nonce: current_loop_nonce.as_u64(), 
            code_hash: REVM_KECCAK_EMPTY,
            code: None,
        };
        cache_db.insert_account_info(caller_revm_address, account_info.clone()); // Clone account_info
        info!("Inserted balance (100 ETH) and nonce ({}) for caller {} into CacheDB for tx {}.",
            current_loop_nonce.as_u64(), caller_revm_address, i + 1);

        // 2. Craft Transaction & Sign
        let calldata: EthersBytes;
        let target_contract_address: EthersAddress;
        let gas_limit: EthersU256;

        if i == 0 { // First transaction: Approve USDC for Uniswap Router
            info!("Tx {} is an APPROVE transaction (USDC -> Uniswap Router)", i + 1);
            target_contract_address = usdc_ethers_address;
            // Approve 1,000,000 USDC (1000 * 10^6, since USDC has 6 decimals)
            let approve_amount_usdc = EthersU256::from(1000) * EthersU256::from(10).pow(EthersU256::from(6));
            calldata = build_approve_calldata(uniswap_router_ethers_address, approve_amount_usdc);
            gas_limit = EthersU256::from(65_000); // Typical gas for approve
        } else { // Second transaction: Swap USDC for WETH via Uniswap Router
            info!("Tx {} is a SWAP transaction (USDC -> WETH via Uniswap Router)", i + 1);
            target_contract_address = uniswap_router_ethers_address;
            let amount_in_usdc = EthersU256::from(1000) * EthersU256::from(10).pow(EthersU256::from(6)); // 1000 USDC
            let amount_out_min_weth = EthersU256::zero(); // For simplicity, no slippage protection
            let path = vec![usdc_ethers_address, weth_ethers_address];
            let to_recipient_address = caller_ethers_address; // Swap sends WETH back to caller
            // Deadline: 15 minutes from now
            let current_timestamp_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?.as_secs();
            let deadline = EthersU256::from(current_timestamp_secs + 15 * 60);

            calldata = build_swap_calldata(
                amount_in_usdc,
                amount_out_min_weth,
                path,
                to_recipient_address,
                deadline,
            );
            gas_limit = EthersU256::from(250_000); // Higher gas for swap
        }

        let tx_request = TransactionRequest {
            from: Some(caller_ethers_address),
            to: Some(NameOrAddress::Address(target_contract_address)), // Use the determined target
            gas: Some(gas_limit), // Use the determined gas limit
            gas_price: Some(EthersU256::from_dec_str("20000000000")? + EthersU256::from(i * 100000000)), // Vary gas price slightly
            value: Some(EthersU256::zero()),
            data: Some(calldata.clone()),
            nonce: Some(current_loop_nonce),
            chain_id: Some(EthersU64::from(chain_id_u64)),
            ..Default::default()
        };

        let typed_tx = TypedTransaction::Legacy(tx_request.clone());
        let signature_ethers = wallet.sign_transaction(&typed_tx).await?;
        let raw_rlp_bytes: EthersBytes = typed_tx.rlp_signed(&signature_ethers);
        info!("Tx {} Raw RLP (ethers-rs): 0x{}", i + 1, hex::encode(&raw_rlp_bytes));

        // 3. Decode RLP & Recover Sender
        let rlp = ethers_core::utils::rlp::Rlp::new(&raw_rlp_bytes);
        let decoded_ethers_tx = EthersTransaction::decode(&rlp)?;
        let tx_hash_ethers = decoded_ethers_tx.hash(); // Calculate transaction hash
        info!("Tx {} Hash (Ethers H256): {:?}", i + 1, tx_hash_ethers); // Log the transaction hash

        let recovered_sender_ethers_address = decoded_ethers_tx.recover_from()?;
        if caller_ethers_address != recovered_sender_ethers_address {
            error!("Tx {} Caller and recovered sender mismatch! Original: {:?}, Recovered: {:?}", i + 1, caller_ethers_address, recovered_sender_ethers_address);
            return Err(anyhow!("Tx {} Caller and recovered sender mismatch!", i + 1));
        }

        // 4. Populate TxEnv
        let mut tx_env = TxEnv::default();
        tx_env.caller = caller_revm_address;
        tx_env.gas_limit = decoded_ethers_tx.gas.as_u64();
        tx_env.gas_price = ethers_u256_to_u128_safe(decoded_ethers_tx.gas_price.unwrap_or_default());
        tx_env.gas_priority_fee = decoded_ethers_tx.max_priority_fee_per_gas.map(ethers_u256_to_u128_safe);
        tx_env.kind = match decoded_ethers_tx.to {
            Some(addr) => RevmTxKind::Call(ethers_to_revm_address(addr)),
            None => RevmTxKind::Create,
        };
        tx_env.value = ethers_to_revm_u256(decoded_ethers_tx.value);
        tx_env.data = RevmBytes(decoded_ethers_tx.input.0.clone());
        tx_env.nonce = decoded_ethers_tx.nonce.as_u64(); // TxEnv nonce is u64
        tx_env.chain_id = decoded_ethers_tx.chain_id.map(|id| id.as_u64()); // TxEnv chain_id is Option<u64>
        info!("Tx {} Populated TxEnv: {:?}", i + 1, tx_env);

        // 5. Setup REVM Environments
        let mut cfg_env = CfgEnv::default();
        cfg_env.chain_id = chain_id_u64;
        cfg_env.spec = RevmSpecId::SHANGHAI;

        let latest_block_ethers = client.get_block(EthersBlockId::Number(EthersBlockNumber::Latest)).await?
            .ok_or_else(|| anyhow!("Failed to get latest block for tx {}", i+1))?;
        let mut block_env = BlockEnv::default();
        
        // Correctly convert Option<U64> to EthersU256 then to RevmU256
        let block_num_ethers_u64 = latest_block_ethers.number.unwrap_or_default();
        block_env.number = ethers_to_revm_u256(EthersU256::from(block_num_ethers_u64.as_u64()));
        block_env.timestamp = ethers_to_revm_u256(latest_block_ethers.timestamp);
        block_env.beneficiary = latest_block_ethers.author.map_or(RevmAddress::ZERO, ethers_to_revm_address);
        block_env.difficulty = ethers_to_revm_u256(latest_block_ethers.difficulty);
        block_env.prevrandao = latest_block_ethers.mix_hash.map(|h: EthersH256| RevmB256::from_slice(h.as_bytes()));
        block_env.basefee = ethers_u256_to_u64_safe(latest_block_ethers.base_fee_per_gas.unwrap_or_default());
        block_env.gas_limit = ethers_u256_to_u64_safe(latest_block_ethers.gas_limit);
        info!("Tx {} Populated BlockEnv (from latest block): {:?}", i + 1, block_env);

        // 6. Execute Transaction
        let mut evm_context: RevmContext<
            BlockEnv, 
            TxEnv, 
            CfgEnv, 
            RevmAlloyCacheDB, // Use the type alias
            Journal<RevmAlloyCacheDB>,
            ()
        > = RevmContext::new(cache_db, cfg_env.spec);

        evm_context.cfg = cfg_env;
        evm_context.block = block_env;
        evm_context.tx = tx_env.clone();

        let mut mainnet_evm = evm_context.build_mainnet();
        match mainnet_evm.transact_commit(tx_env) {
            Ok(transaction_outcome) => {
                info!("Tx {} Full REVM Transaction Outcome (ExecutionResult): {:?}", i + 1, transaction_outcome);

                match transaction_outcome {
                    ExecutionResult::Success { reason, gas_used, gas_refunded, logs, output } => {
                        info!("Tx {} -> Success: {:?}", i + 1, reason);
                        info!("Tx {} -> Gas Used: {}", i + 1, gas_used);
                        info!("Tx {} -> Gas Refunded: {}", i + 1, gas_refunded);
                        info!("Tx {} -> Output: {:?}", i + 1, output);

                        info!("Tx {} -> Event Logs Emitted: {}", i + 1, logs.len());
                        for (log_idx, log_entry) in logs.iter().enumerate() {
                            info!("Tx {}   Log #{}: Address: {:?}, Topics: {:?}, Data: 0x{}",
                                i + 1,
                                log_idx + 1,
                                log_entry.address,
                                log_entry.topics(),
                                hex::encode(&log_entry.data.data)
                            );
                        }
                    }
                    ExecutionResult::Revert { gas_used, output } => {
                        info!("Tx {} -> Reverted. Gas Used: {}, Output: {:?}", i + 1, gas_used, output);
                    }
                    ExecutionResult::Halt { reason, gas_used } => {
                        info!("Tx {} -> Halted: {:?}. Gas Used: {}", i + 1, reason, gas_used);
                    }
                }

                // Iterate over the accounts in the CacheDB to log their state.
                // mainnet_evm.ctx.db() provides access to the CacheDB
                if mainnet_evm.ctx.db().cache.accounts.is_empty() {
                    info!("Tx {}     No accounts found in CacheDB state after transaction. This might be unexpected if the caller was pre-loaded.", i + 1);
                } else {
                    for (address, account_state) in mainnet_evm.ctx.db().cache.accounts.iter() {
                        // account_state is &revm_primitives::Account
                        // We only want to log accounts that were touched or had storage changes.
                        // For simplicity here, we'll log all accounts present in the cache post-tx.
                        // A more advanced version could compare with a pre-transaction snapshot.

                        info!("Tx {}     Account: {:?}, Status: {:?}", i + 1, address, account_state.account_state);
                        info!("Tx {}       Info: Balance: {}, Nonce: {}, CodeHash: {:?}, Code Loaded: {}",
                            i + 1,
                            account_state.info.balance,
                            account_state.info.nonce,
                            account_state.info.code_hash,
                            account_state.info.code.is_some()
                        );

                        if account_state.account_state == revm::database::AccountState::StorageCleared {
                            info!("Tx {}         Account was self-destructed (StorageCleared).", i+1);
                        }

                        if account_state.storage.is_empty() {
                            info!("Tx {}       Storage: No storage slots present in cache for this account.", i + 1);
                        } else {
                            info!("Tx {}       Storage (Cached after Tx):", i + 1);
                            for (slot, value) in account_state.storage.iter() {
                                info!("Tx {}         Slot: {:?} -> Value: {:?}",
                                    i + 1,
                                    slot,
                                    value
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Tx {} REVM Execution Failed: {:?}", i + 1, e);
                // Decide if you want to continue to the next tx or stop
            }
        }
        info!("--- Finished Simulating Transaction {}/{} ---", i + 1, NUM_TX_TO_SIMULATE);
    } // End of simulation loop

    info!("Completed simulation of {} transactions.", NUM_TX_TO_SIMULATE);
    Ok(())
} 