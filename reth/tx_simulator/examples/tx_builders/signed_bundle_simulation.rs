//! Demonstrates building and simulating a signed transaction bundle.
//!
//! Flow:
//! 1. Buy token on Uniswap V2 (ETH -> USDC)
//! 2. Approve Sushi router to spend USDC
//! 3. Sell USDC back to ETH on Sushi V2
//!
//! This example showcases how the tx_builders helpers can be paired with the
//! simulator to verify complex transaction chains before broadcasting.

use alloy_consensus::{SignableTransaction, TxKind, TxLegacy};
use alloy_primitives::{utils::parse_ether, B256};
use alloy_primitives::{Address as AlloyAddress, U256 as AlloyU256};
use reth_chain_query::dex::compute_sushiswap_pool;
use reth_ethereum_primitives::{Transaction, TransactionSigned};
use reth_primitives_traits::crypto::secp256k1::{recover_signer_unchecked, sign_message};
use std::env;
use tx_simulator::tx_builders::{
    amm_swap_route::AmmSwapRoute, build_approve_for_route, build_buy_swap_with_min_out,
    build_sell_swap_with_min_out,
};
use tx_simulator::{SignedTxChainSimulation, TxSimulator};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Reth DB path is either provided via env or defaults to ~/.local/share/reth/mainnet
    let reth_db = env::var("RETH_DB_PATH").unwrap_or_else(|_| {
        format!(
            "{}/.local/share/reth/mainnet",
            std::env::var("HOME").unwrap_or_else(|_| "/home/nima".into())
        )
    });
    let simulator = TxSimulator::new(&reth_db)?;
    println!(
        "Reth DB: {} | latest block {}",
        reth_db,
        simulator.get_latest_block()?
    );

    // Wallet used purely for simulation (never broadcast!)
    let pk = env::var("KARTAL_KILIT").expect("Set KARTAL_KILIT to a dev private key");
    let signer_secret = parse_private_key(&pk)?;
    let chain_id: u64 = env::var("ETH_KARTAL_CHAIN_ID")
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
    let eth_before = chain.eth_balance_of_on_fork(owner)?;

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
    format!("0x{key}")
        .parse::<B256>()
        .map_err(|e| eyre::eyre!("invalid KARTAL_KILIT hex private key: {}", e))
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
