//! Demonstrates building and simulating a signed transaction bundle.
//!
//! Flow:
//! 1. Buy token on Uniswap V2 (ETH -> USDC)
//! 2. Approve Sushi router to spend USDC
//! 3. Sell USDC back to ETH on Sushi V2
//!
//! This example showcases how the tx_builders helpers can be paired with the
//! simulator to verify complex transaction chains before broadcasting.

use alloy_consensus::{SignableTransaction, TxLegacy};
use alloy_primitives::U256 as AlloyU256;
use alloy_primitives::{
    address, keccak256, utils::parse_ether, Address as AlloyAddress, TxKind, B256,
};
use reth_ethereum_primitives::{Transaction, TransactionSigned};
use reth_primitives_traits::crypto::secp256k1::{recover_signer_unchecked, sign_message};
use std::env;
use tx_simulator::tx_builders::{
    amm_swap_route::AmmSwapRoute, build_approve_for_route, build_buy_swap_with_min_out,
    build_sell_swap_with_min_out,
};
use tx_simulator::{SignedTxChainSimulation, TxSimulator};

const SUSHISWAP_FACTORY: AlloyAddress = address!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac");
const SUSHISWAP_INIT_CODE_HASH: [u8; 32] = [
    0xe1, 0x8a, 0x34, 0xeb, 0x0e, 0x04, 0xb0, 0x4f, 0x7a, 0x0a, 0xc2, 0x9a, 0x6e, 0x80, 0x74, 0x8d,
    0xca, 0x96, 0x31, 0x9b, 0x42, 0xc5, 0x4d, 0x67, 0x9c, 0xb8, 0x21, 0xdc, 0xa9, 0x0c, 0x63, 0x03,
];

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Wallet used purely for simulation (never broadcast!)
    let pk = match env::var("ETH_TX_EXECUTOR_DEV_PRIVATE_KEY") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => {
            println!("Skipping signed bundle simulation: set ETH_TX_EXECUTOR_DEV_PRIVATE_KEY to a dev private key.");
            return Ok(());
        }
        Err(err) => {
            return Err(eyre::eyre!(
                "failed to read ETH_TX_EXECUTOR_DEV_PRIVATE_KEY: {}",
                err
            ))
        }
    };
    let signer_secret = parse_private_key(&pk)?;

    // Reth DB path is resolved from env overrides or the workspace config.
    let reth_db = tx_simulator::config::repo::reth_datadir()?;
    let simulator = TxSimulator::new(&reth_db)?;
    println!(
        "Reth DB: {} | latest block {}",
        reth_db,
        simulator.get_latest_block()?
    );

    let chain_id: u64 = env::var("ETH_TX_EXECUTOR_CHAIN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let owner = private_key_to_address(signer_secret)?;
    println!("Wallet: {:?}", owner);

    // Addresses/constants
    let weth: AlloyAddress = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        .parse()
        .unwrap();
    let usdc: AlloyAddress = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        .parse()
        .unwrap();
    let uni_v2_usdc_weth: AlloyAddress = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
        .parse()
        .unwrap();
    let sushi_v2_usdc_weth: AlloyAddress = compute_sushiswap_pool(weth, usdc);
    let uni_route = AmmSwapRoute::UniswapV2 {
        pool: uni_v2_usdc_weth,
    };
    let sushi_route = AmmSwapRoute::SushiswapV2 {
        pool: sushi_v2_usdc_weth,
    };

    // Trade params
    let amount_in_eth =
        parse_ether(&env::var("TRADE_AMOUNT_ETH").unwrap_or_else(|_| "0.01".into()))?;

    let latest_block = simulator.get_latest_block()?;
    let base_fee_wei = simulator
        .get_base_fee_at_block(latest_block)
        .unwrap_or(10_000_000_000u128); // fallback 10 gwei
    let tip_gwei: u64 = env::var("PRIORITY_FEE_GWEI")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let env_gas_gwei: Option<u64> = env::var("GAS_PRICE_GWEI").ok().and_then(|v| v.parse().ok());
    let computed_gas = base_fee_wei + (tip_gwei as u128) * 1_000_000_000u128;
    let gas_price_wei: u128 = match env_gas_gwei {
        Some(g) => (g as u128) * 1_000_000_000u128,
        None => computed_gas,
    };
    println!(
        "Block {} base_fee: {} wei | chosen gas_price: {} wei",
        latest_block, base_fee_wei, gas_price_wei
    );
    let gas_approve: u64 = 120_000;
    let gas_swap: u64 = 300_000;

    // Create sim chain and get starting nonce
    let mut chain: SignedTxChainSimulation = simulator.start_signed_chain(None)?;
    let mut nonce = chain.nonce_of(owner)?;

    // Record ETH balance before buy
    let mut eth_before = chain.eth_balance_of_on_fork(owner)?;
    let estimated_gas_cost =
        AlloyU256::from(gas_price_wei) * AlloyU256::from(gas_swap * 2 + gas_approve);
    let required_balance = amount_in_eth + estimated_gas_cost;
    if eth_before < required_balance {
        let funded_balance = required_balance * AlloyU256::from(2);
        let previous = chain.set_eth_balance_on_fork(owner, funded_balance)?;
        println!(
            "Funded signer on fork for simulation only: previous {} wei, forked {} wei",
            previous, funded_balance
        );
        eth_before = funded_balance;
    }

    // Build and sign BUY (ETH->USDC)
    let mut buy_unsigned = build_buy_swap_with_min_out(
        &uni_route,
        owner,
        usdc,
        amount_in_eth,
        AlloyU256::ZERO,
        u64::MAX,
    );
    buy_unsigned.from = Some(owner);
    buy_unsigned.gas = Some(gas_swap);
    buy_unsigned.gas_price = Some(gas_price_wei);
    buy_unsigned.nonce = Some(nonce);
    let signed_buy = sign_unsigned_legacy(signer_secret, chain_id, buy_unsigned)?;

    let r1 = chain.step(&signed_buy)?;
    let usdc_after_buy = chain.erc20_balance_of_on_fork(usdc, owner)?;
    let eth_after_buy = chain.eth_balance_of_on_fork(owner)?;
    println!(
        "Buy success: {} | gas_used: {} | USDC after buy: {} | ETH before: {} | ETH after buy: {}",
        r1.success, r1.gas_used, usdc_after_buy, eth_before, eth_after_buy
    );

    // Approve MAX for Sushi router
    nonce += 1;
    let mut approve_unsigned = build_approve_for_route(&sushi_route, owner, usdc, AlloyU256::MAX);
    approve_unsigned.from = Some(owner);
    approve_unsigned.gas = Some(gas_approve);
    approve_unsigned.gas_price = Some(gas_price_wei);
    approve_unsigned.nonce = Some(nonce);
    let signed_approve = sign_unsigned_legacy(signer_secret, chain_id, approve_unsigned)?;
    let r2 = chain.step(&signed_approve)?;
    println!(
        "Approve success: {} | gas_used: {}",
        r2.success, r2.gas_used
    );

    // Sell USDC -> ETH on Sushi V2
    nonce += 1;
    let mut sell_unsigned = build_sell_swap_with_min_out(
        &sushi_route,
        owner,
        usdc,
        usdc_after_buy,
        AlloyU256::ZERO,
        u64::MAX,
    );
    sell_unsigned.from = Some(owner);
    sell_unsigned.gas = Some(gas_swap);
    sell_unsigned.gas_price = Some(gas_price_wei);
    sell_unsigned.nonce = Some(nonce);
    let signed_sell = sign_unsigned_legacy(signer_secret, chain_id, sell_unsigned)?;
    let r3 = chain.step(&signed_sell)?;
    let usdc_after_sell = chain.erc20_balance_of_on_fork(usdc, owner)?;
    let eth_after_sell = chain.eth_balance_of_on_fork(owner)?;
    println!(
        "Sell success: {} | gas_used: {} | USDC after sell: {} | ETH after sell: {}",
        r3.success, r3.gas_used, usdc_after_sell, eth_after_sell
    );

    Ok(())
}

fn parse_private_key(raw: &str) -> eyre::Result<B256> {
    let key = raw.trim().trim_start_matches("0x");
    format!("0x{key}").parse::<B256>().map_err(|e| {
        eyre::eyre!(
            "invalid ETH_TX_EXECUTOR_DEV_PRIVATE_KEY hex private key: {}",
            e
        )
    })
}

fn private_key_to_address(secret: B256) -> eyre::Result<AlloyAddress> {
    let probe = B256::from([1u8; 32]);
    let sig = sign_message(secret, probe)
        .map_err(|e| eyre::eyre!("failed to derive address from private key: {}", e))?;
    recover_signer_unchecked(&sig, probe)
        .map_err(|e| eyre::eyre!("failed to recover address from signature: {}", e))
}

fn sign_unsigned_legacy(
    signer_secret: B256,
    chain_id: u64,
    unsigned: tx_simulator::UnsignedTransaction,
) -> eyre::Result<TransactionSigned> {
    if unsigned.max_fee_per_gas.is_some()
        || unsigned.max_priority_fee_per_gas.is_some()
        || !unsigned.access_list.is_empty()
        || !unsigned.blob_versioned_hashes.is_empty()
        || unsigned.max_fee_per_blob_gas.is_some()
        || !unsigned.signed_authorizations.is_empty()
    {
        return Err(eyre::eyre!(
            "this example signs legacy transactions only (gas_price path)"
        ));
    }

    let tx = Transaction::Legacy(TxLegacy {
        chain_id: Some(chain_id),
        nonce: unsigned
            .nonce
            .ok_or_else(|| eyre::eyre!("missing nonce on unsigned transaction"))?,
        gas_price: unsigned
            .gas_price
            .ok_or_else(|| eyre::eyre!("missing gas_price on unsigned transaction"))?,
        gas_limit: unsigned
            .gas
            .ok_or_else(|| eyre::eyre!("missing gas limit on unsigned transaction"))?,
        to: unsigned.to.map(TxKind::Call).unwrap_or(TxKind::Create),
        value: unsigned.value.unwrap_or_default(),
        input: unsigned.data.unwrap_or_default(),
    });

    let signature = sign_message(signer_secret, tx.signature_hash())
        .map_err(|e| eyre::eyre!("failed to sign transaction: {}", e))?;
    Ok(TransactionSigned::new_unhashed(tx, signature))
}

fn compute_sushiswap_pool(token_a: AlloyAddress, token_b: AlloyAddress) -> AlloyAddress {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(SUSHISWAP_FACTORY, token0, token1, SUSHISWAP_INIT_CODE_HASH)
}

fn sort_tokens(token_a: AlloyAddress, token_b: AlloyAddress) -> (AlloyAddress, AlloyAddress) {
    if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    }
}

fn compute_create2_address(
    factory: AlloyAddress,
    token0: AlloyAddress,
    token1: AlloyAddress,
    init_code_hash: [u8; 32],
) -> AlloyAddress {
    let mut salt_input = Vec::with_capacity(40);
    salt_input.extend_from_slice(token0.as_slice());
    salt_input.extend_from_slice(token1.as_slice());
    let salt = keccak256(&salt_input);

    let mut input = Vec::with_capacity(85);
    input.push(0xff);
    input.extend_from_slice(factory.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(&init_code_hash);

    let hash = keccak256(&input);
    AlloyAddress::from_slice(&hash[12..])
}
