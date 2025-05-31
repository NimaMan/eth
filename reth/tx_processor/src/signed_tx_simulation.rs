//! examples/signed_tx_simulation.rs
//! Simulates a signed Uniswap v2 swap transaction.
//! Objective: Demonstrate crafting, signing (with ethers-rs), RLP decoding (with ethers-rs),
//! manual TxEnv population, and execution with a local REVM clone.
//!
//! Algorithm:
//! 1. Craft Transaction & Sign (ethers-rs):
//!    - Define transaction parameters (to, value, data, gas, gasPrice, nonce, chainId).
//!    - Create an `ethers_core::types::TransactionRequest`.
//!    - Sign it using `ethers_signers::LocalWallet` to get a signature.
//!    - Construct a `TypedTransaction` (e.g., `Legacy`) and get the RLP-encoded signed transaction bytes
//!      using `typed_tx.rlp_signed(&signature)`.
//! 2. Decode RLP & Recover Sender (ethers-rs):
//!    - Create an `ethers_core::utils::rlp::Rlp` instance from the raw RLP bytes.
//!    - Decode it into an `ethers_core::types::Transaction` using `Transaction::decode(&rlp)`.
//!    - Recover the sender's `EthersAddress` using `decoded_tx.recover_from()?`.
//! 3. Populate `revm::primitives::TxEnv` (Manual Mapping):
//!    - Initialize a `TxEnv` struct.
//!    - Map fields from the ethers `Transaction` and recovered sender to `TxEnv` fields,
//!      performing necessary type conversions (e.g., EthersU256 to RevmU256, EthersAddress to RevmAddress).
//! 4. Setup REVM & Execute:
//!    - Create `CacheDB` with `EmptyDB`.
//!    - Insert account info for the sender.
//!    - Build `RevmContext` using the builder pattern.
//!    - Set `evm.block_env` and `evm.cfg_env`.
//!    - Assign the manually populated `tx_env` to `evm.tx_env`.
//!    - Call `evm.transact_commit()`.

#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use anyhow::Result;
use ethers_core::{
    types::{
        transaction::eip2718::TypedTransaction, Address as EthersAddress, Bytes as EthersBytes,
        NameOrAddress, Transaction as EthersTransaction,
        TransactionRequest, U256 as EthersU256, H256 as EthersH256,
    },
    utils::rlp::{Decodable as EthersDecodable},
};
use ethers_signers::{LocalWallet, Signer};

// REVM types - using direct sub-crate imports and specific module paths
use revm_database::in_memory_db::CacheDB;
use revm_state::AccountInfo;
use revm_database_interface::EmptyDB; // Infallible is from core::convert

use revm_primitives::{
    Address as RevmAddress, Bytes as RevmBytes, // Removed Env as RevmEnv for now
    TxKind as RevmTxKind, U256 as RevmU256, B256 as RevmB256, KECCAK_EMPTY,
};
use revm_context::{
    BlockEnv, CfgEnv, Context as RevmContext, Journal, TxEnv // These Env components are from context
};
use revm::handler::{ExecuteCommitEvm, MainBuilder}; // Use MainBuilder trait and ExecuteCommitEvm

// std
use std::str::FromStr;


/// build calldata for Uniswap-V2 `swapExactTokensForTokens(uint256,uint256,address[],address,uint256)`
/// For this example, we'll use a simpler swap or make one up, as the original example used a complex one.
/// Let's use `swap(uint amount0Out, uint amount1Out, address to, bytes data)` from UniswapV2Pair
/// Selector: 0x022c0d9f
fn build_simple_swap_calldata(
    amount0_out: EthersU256,
    amount1_out: EthersU256,
    to: EthersAddress,
    // `data` field for swap is often empty or for callbacks, let's use empty.
) -> EthersBytes {
    // function selector 0x022c0d9f
    // (from revm/examples/uniswap_v2_usdc_swap)
    let mut data = hex::decode("022c0d9f").unwrap();
    let mut word = [0u8; 32];

    amount0_out.to_big_endian(&mut word);
    data.extend_from_slice(&word);

    word.fill(0); // Clear for next param
    amount1_out.to_big_endian(&mut word);
    data.extend_from_slice(&word);

    word.fill(0); // Clear for next param
    word[12..].copy_from_slice(to.as_bytes()); // EthersAddress is H160 (20 bytes), pad to 32 bytes
    data.extend_from_slice(&word);

    // For `bytes data` parameter:
    // Offset to data (relative to start of arguments block)
    // (amount0Out (32) + amount1Out (32) + to (32)) = 96 bytes = 0x60
    // Length of data (0 for empty bytes)
    word.fill(0);
    EthersU256::from(128).to_big_endian(&mut word); // Offset to the `data` parameter (points to after itself)
    data.extend_from_slice(&word);

    word.fill(0);
    EthersU256::from(0).to_big_endian(&mut word); // Length of the `data` parameter (0 bytes)
    data.extend_from_slice(&word);
    // No actual data for the `data` parameter itself as length is 0.

    EthersBytes::from(data)
}


// Helper to convert EthersU256 to RevmU256
fn ethers_to_revm_u256(val: EthersU256) -> RevmU256 {
    let mut bytes = [0u8; 32];
    val.to_big_endian(&mut bytes);
    RevmU256::from_be_bytes(bytes)
}

// Helper to convert EthersU256 to u128 for gas fields if direct conversion is not available
// Note: This might truncate if EthersU256 is larger than u128::MAX
fn ethers_u256_to_u128_safe(val: EthersU256) -> u128 {
    if val > EthersU256::from(u128::MAX) {
        // Or handle error: panic!("EthersU256 value too large for u128");
        u128::MAX 
    } else {
        val.as_u128()
    }
}

// Helper to convert EthersU256 to u64 for nonce and other fields
// Note: This might truncate.
fn ethers_u256_to_u64_safe(val: EthersU256) -> u64 {
    if val > EthersU256::from(u64::MAX) {
        u64::MAX
    } else {
        val.as_u64()
    }
}

// Helper to convert EthersAddress to RevmAddress
fn ethers_to_revm_address(addr: EthersAddress) -> RevmAddress {
    RevmAddress::from_slice(addr.as_bytes())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init(); // Initialize tracing

    //------------------------------------------------------------------#
    // 0.  Test credentials & addresses                                 #
    //------------------------------------------------------------------#
    // (Taken from Anvil's default private keys for easy testing if needed locally)
    const PRIVKEY_HEX: &str = "59c6995e998f97a5a004498123312aaab5e367d6f4e97d96b2ebe497acf7f5b1";
    let wallet: LocalWallet = PRIVKEY_HEX.parse::<LocalWallet>()?.with_chain_id(1u64); // Chain ID 1 (Mainnet)

    let caller_ethers_address = wallet.address();
    // A known contract address (e.g., Uniswap V2 Router, or a dummy one for this test)
    // For simplicity, let's use the pair address from the original uniswap_v2_usdc_swap example
    // which is 0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc (USDC/WETH)
    // but the calldata is for pair.swap, so this should be a PAIR address.
    // The original example in revm used a pair address derived from token addresses.
    // Let's use a placeholder that's a valid address format.
    let contract_ethers_address = EthersAddress::from_str("0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B")?; // A random valid address

    println!("Using Caller Address (Ethers): {:?}", caller_ethers_address);
    println!("Using Contract Address (Ethers): {:?}", contract_ethers_address);

    //------------------------------------------------------------------#
    // 1.  Craft Transaction & Sign (ethers-rs)                         #
    //------------------------------------------------------------------#
    let amount_out0 = EthersU256::from_dec_str("1000000000000000000")?; // 1 token (assuming 18 decimals)
    let calldata = build_simple_swap_calldata(
        amount_out0,
        EthersU256::zero(), // amount_out1
        caller_ethers_address, // 'to' address for swapped tokens
    );

    let tx_request = TransactionRequest {
        from: Some(caller_ethers_address),
        to: Some(NameOrAddress::Address(contract_ethers_address)),
        gas: Some(EthersU256::from(210000)), // Typical gas limit for a swap
        gas_price: Some(EthersU256::from_dec_str("20000000000")?), // 20 Gwei
        value: Some(EthersU256::zero()), // No ETH sent with the swap call itself
        data: Some(calldata.clone()),
        nonce: Some(EthersU256::from(0)), // Example nonce
        chain_id: Some(1u64.into()),      // Mainnet
        ..Default::default() // transaction_type will be None (Legacy) by default
    };

    let typed_tx = TypedTransaction::Legacy(tx_request.clone()); // Or Eip1559, etc.
    let signature_ethers = wallet.sign_transaction(&typed_tx).await?;
    let raw_rlp_bytes: EthersBytes = typed_tx.rlp_signed(&signature_ethers);

    println!("Raw RLP (ethers-rs): 0x{}", hex::encode(&raw_rlp_bytes));
    println!("Signature (ethers-rs): r: {}, s: {}, v: {}", signature_ethers.r, signature_ethers.s, signature_ethers.v);


    //------------------------------------------------------------------#
    // 2.  Decode RLP & Recover Sender (ethers-rs)                      #
    //------------------------------------------------------------------#
    let rlp = ethers_core::utils::rlp::Rlp::new(&raw_rlp_bytes);
    let decoded_ethers_tx = EthersTransaction::decode(&rlp)?;

    let recovered_sender_ethers_address = decoded_ethers_tx.recover_from()?;
    println!(
        "Decoded Tx Sender (Ethers, recovered): {:?}",
        recovered_sender_ethers_address
    );
    assert_eq!(
        caller_ethers_address, recovered_sender_ethers_address,
        "Original caller and recovered sender mismatch!"
    );

    //------------------------------------------------------------------#
    // 3.  Populate `revm::primitives::TxEnv` (Manual Mapping)          #
    //------------------------------------------------------------------#
    let mut tx_env = TxEnv::default(); // Start with revm's default

    tx_env.caller = ethers_to_revm_address(recovered_sender_ethers_address);
    tx_env.gas_limit = decoded_ethers_tx.gas.as_u64(); // gas_limit is u64
    tx_env.gas_price = ethers_u256_to_u128_safe(decoded_ethers_tx.gas_price.unwrap_or_default());
    tx_env.gas_priority_fee = decoded_ethers_tx.max_priority_fee_per_gas.map(ethers_u256_to_u128_safe);

    // Use `kind` field instead of `transact_to`
    tx_env.kind = match decoded_ethers_tx.to {
        Some(addr) => RevmTxKind::Call(ethers_to_revm_address(addr)),
        None => RevmTxKind::Create, // Should not happen for this example
    };
    tx_env.value = ethers_to_revm_u256(decoded_ethers_tx.value);
    tx_env.data = RevmBytes(decoded_ethers_tx.input.0.clone());
    // Nonce is now u64, not Option<u64>
    tx_env.nonce = decoded_ethers_tx.nonce.as_u64(); 
    tx_env.chain_id = decoded_ethers_tx.chain_id.map(|id| id.as_u64()); // chain_id in TxEnv is Option<u64>

    // EIP-4844 fields (not used in legacy tx)
    // tx_env.blob_hashes = Vec::new();
    // tx_env.max_fee_per_blob_gas = None; // RevmU256::ZERO; // Check TxEnv definition

    // EIP-7702 fields (not used in legacy tx)
    // tx_env.authorization_list = None; // Option<Vec<Authorization>>

    // tx_type is implicitly handled by REVM based on fields present or can be set if needed
    // For legacy, it's usually 0. `decoded_ethers_tx.transaction_type` would be None.

    println!("Populated TxEnv: {:?}", tx_env);

    //------------------------------------------------------------------#
    // 4.  Setup REVM & Execute                                         #
    //------------------------------------------------------------------#
    let mut cache_db = CacheDB::new(EmptyDB::default());

    // Fund the caller's account
    let caller_revm_address = ethers_to_revm_address(caller_ethers_address);
    let account_info = AccountInfo {
        balance: ethers_to_revm_u256(EthersU256::from_dec_str("100000000000000000000")?), // 100 ETH
        nonce: 0, // Initial nonce before this tx (tx_env.nonce will be 0 for the first tx)
        code_hash: KECCAK_EMPTY, // No code for EOA
        code: None,              // No code for EOA
    };
    cache_db.insert_account_info(caller_revm_address, account_info);

    // For the contract we are calling, we might need to insert its code
    // For this example, let's assume it's an EOA or we don't care about its code execution path deeply,
    // or that EmptyDB will treat calls to non-existent accounts as successful no-ops (depending on spec)
    // If it were a real contract, we'd use:
    // let contract_revm_address = ethers_to_revm_address(contract_ethers_address);
    // let contract_code = RevmBytes::from_str("...")?; // Actual bytecode
    // cache_db.insert_account_info(contract_revm_address, AccountInfo {
    //     code: Some(contract_code.clone()), // Note: REVM's AccountInfo takes Option<Bytecode>
    //     code_hash: Bytecode::new_raw(contract_code).hash_slow(),
    //     ..Default::default()
    // });


    // EVM setup using individual environment components
    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = 1u64; // chain_id is u64
    // Use `spec` field and fully qualified `SpecId`
    cfg_env.spec = revm_primitives::hardfork::SpecId::SHANGHAI; 

    let mut block_env = BlockEnv::default();
    block_env.number = RevmU256::from(1_000_000); // number is U256
    block_env.timestamp = RevmU256::from(block_timestamp_now()); // timestamp is U256
    // Use `beneficiary` field instead of `coinbase`
    block_env.beneficiary = RevmAddress::ZERO; 
    block_env.difficulty = RevmU256::ZERO; // difficulty is U256
    block_env.prevrandao = Some(RevmB256::ZERO);
    // basefee is u64
    block_env.basefee = ethers_u256_to_u64_safe(EthersU256::from_dec_str("10000000000")?); 
    // gas_limit is u64
    block_env.gas_limit = 30_000_000u64; 

    // Our tx_env is already prepared

    // Use `cfg_env.spec` for the second argument to RevmContext::new
    // Provide explicit type for evm_context to help with type inference
    let mut evm_context: RevmContext<
        BlockEnv, 
        TxEnv, 
        CfgEnv, 
        CacheDB<EmptyDB>,
        Journal<CacheDB<EmptyDB>>,
        ()
    > = RevmContext::new(cache_db, cfg_env.spec);

    // Set the environments directly on the context fields
    evm_context.cfg = cfg_env;
    evm_context.block = block_env;
    evm_context.tx = tx_env.clone(); // CLONE tx_env for the context

    // Create a MainnetEvm handler using the MainBuilder trait on our context
    let mut mainnet_evm = evm_context.build_mainnet();

    // Transact through the MainnetEvm handler, passing the original tx_env
    let result_and_state = mainnet_evm.transact_commit(tx_env)?; // Pass original tx_env by value

    println!("Full REVM Execution Result & State: {:?}", result_and_state);

    // Assuming ExecutionResult directly contains the outcome fields/methods
    println!(" -> Success: {}", result_and_state.is_success());
    println!(" -> Gas Used: {}", result_and_state.gas_used());
    println!(" -> Output: {:?}", result_and_state.output());
    println!(" -> Logs: {:?}", result_and_state.logs());
    // println!(" -> State Changes: {:?}", result_and_state.state); // State might be large

    // Check the balance of the USDC contract after the swap (example of checking state)

    Ok(())
}

fn block_timestamp_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}