//! Demonstrates building and simulating a signed transaction bundle.
//!
//! Flow:
//! 1. Buy token on Uniswap V2 (ETH -> USDC)
//! 2. Approve Sushi router to spend USDC
//! 3. Sell USDC back to ETH on Sushi V2
//!
//! This example showcases how the tx_builders helpers can be paired with the
//! simulator to verify complex transaction chains before broadcasting.

use alloy_primitives::{Address as AlloyAddress, U256 as AlloyU256};
use alloy_rlp::Decodable;
use ethers::prelude::*;
use ethers::types::Bytes as EthersBytes;
use ethers::utils::parse_units;
use reth_chain_query::dex::compute_sushiswap_pool;
use reth_chain_query::tx_builders::{
    amm_swap_route::AmmSwapRoute, build_approve_for_route, build_buy_swap_with_min_out,
    build_sell_swap_with_min_out,
};
use reth_primitives::TransactionSigned;
use std::env;
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
    let chain_id: u64 = env::var("ETH_KARTAL_CHAIN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let wallet: LocalWallet = pk.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let owner = wallet.address();
    println!("Wallet: {:?}", owner);

    // Addresses/constants
    let weth: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        .parse()
        .unwrap();
    let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        .parse()
        .unwrap();
    let uni_v2_usdc_weth: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
        .parse()
        .unwrap();
    let sushi_v2_usdc_weth: Address = alloy_to_ethers_addr(compute_sushiswap_pool(
        ethers_to_alloy_addr(weth),
        ethers_to_alloy_addr(usdc),
    ));
    let uni_route = AmmSwapRoute::UniswapV2 {
        pool: ethers_to_alloy_addr(uni_v2_usdc_weth),
    };
    let sushi_route = AmmSwapRoute::SushiswapV2 {
        pool: ethers_to_alloy_addr(sushi_v2_usdc_weth),
    };

    // Trade params
    let amount_in_eth: U256 = parse_units(
        env::var("TRADE_AMOUNT_ETH").unwrap_or_else(|_| "0.01".into()),
        "ether",
    )?
    .into();

    let latest_block = simulator.get_latest_block()?;
    let base_fee_wei = simulator
        .get_base_fee_at_block(latest_block)
        .unwrap_or(10_000_000_000u128); // fallback 10 gwei
    let tip_gwei: u64 = env::var("PRIORITY_FEE_GWEI")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let env_gas_gwei: Option<u64> = env::var("GAS_PRICE_GWEI").ok().and_then(|v| v.parse().ok());
    let computed_gas =
        U256::from(base_fee_wei) + U256::from(tip_gwei) * U256::from(1_000_000_000u64);
    let gas_price = match env_gas_gwei {
        Some(g) => U256::from(g) * U256::from(1_000_000_000u64),
        None => computed_gas,
    };
    println!(
        "Block {} base_fee: {} wei | chosen gas_price: {} wei",
        latest_block, base_fee_wei, gas_price
    );
    let gas_approve = U256::from(120_000);
    let gas_swap = U256::from(300_000);

    // Create sim chain and get starting nonce
    let mut chain: SignedTxChainSimulation = simulator.start_signed_chain(None)?;
    let mut nonce = chain.nonce_of(ethers_to_alloy_addr(owner))?;

    // Record ETH balance before buy
    let eth_before = chain.eth_balance_of_on_fork(ethers_to_alloy_addr(owner))?;

    // Build and sign BUY (ETH->USDC)
    let buy_unsigned = build_buy_swap_with_min_out(
        &uni_route,
        ethers_to_alloy_addr(owner),
        ethers_to_alloy_addr(usdc),
        ethers_to_alloy_u256(amount_in_eth),
        AlloyU256::ZERO,
        u64::MAX,
    );
    let mut buy_typed = unsigned_to_typed(buy_unsigned, owner);
    buy_typed.set_gas(gas_swap);
    buy_typed.set_gas_price(gas_price);
    buy_typed.set_nonce(U256::from(nonce));
    let signed_buy = sign_typed(&wallet, &buy_typed)?;

    let r1 = chain.step(&signed_buy)?;
    let usdc_after_buy =
        chain.erc20_balance_of_on_fork(ethers_to_alloy_addr(usdc), ethers_to_alloy_addr(owner))?;
    let eth_after_buy = chain.eth_balance_of_on_fork(ethers_to_alloy_addr(owner))?;
    println!(
        "Buy success: {} | gas_used: {} | USDC after buy: {} | ETH before: {} | ETH after buy: {}",
        r1.success, r1.gas_used, usdc_after_buy, eth_before, eth_after_buy
    );

    // Approve MAX for Sushi router
    nonce += 1;
    let approve_unsigned = build_approve_for_route(
        &sushi_route,
        ethers_to_alloy_addr(owner),
        ethers_to_alloy_addr(usdc),
        AlloyU256::MAX,
    );
    let mut approve_typed = unsigned_to_typed(approve_unsigned, owner);
    approve_typed.set_gas(gas_approve);
    approve_typed.set_gas_price(gas_price);
    approve_typed.set_nonce(U256::from(nonce));
    let signed_approve = sign_typed(&wallet, &approve_typed)?;
    let r2 = chain.step(&signed_approve)?;
    println!(
        "Approve success: {} | gas_used: {}",
        r2.success, r2.gas_used
    );

    // Sell USDC -> ETH on Sushi V2
    nonce += 1;
    let sell_unsigned = build_sell_swap_with_min_out(
        &sushi_route,
        ethers_to_alloy_addr(owner),
        ethers_to_alloy_addr(usdc),
        usdc_after_buy,
        AlloyU256::ZERO,
        u64::MAX,
    );
    let mut sell_typed = unsigned_to_typed(sell_unsigned, owner);
    sell_typed.set_gas(gas_swap);
    sell_typed.set_gas_price(gas_price);
    sell_typed.set_nonce(U256::from(nonce));
    let signed_sell = sign_typed(&wallet, &sell_typed)?;
    let r3 = chain.step(&signed_sell)?;
    let usdc_after_sell =
        chain.erc20_balance_of_on_fork(ethers_to_alloy_addr(usdc), ethers_to_alloy_addr(owner))?;
    let eth_after_sell = chain.eth_balance_of_on_fork(ethers_to_alloy_addr(owner))?;
    println!(
        "Sell success: {} | gas_used: {} | USDC after sell: {} | ETH after sell: {}",
        r3.success, r3.gas_used, usdc_after_sell, eth_after_sell
    );

    Ok(())
}

fn unsigned_to_typed(
    unsigned: tx_simulator::UnsignedTransaction,
    from: ethers::types::Address,
) -> ethers::types::transaction::eip2718::TypedTransaction {
    let mut tx: ethers::types::transaction::eip2718::TypedTransaction =
        (ethers::types::TransactionRequest::new().from(from)).into();
    if let Some(to) = unsigned.to {
        tx.set_to(ethers::types::NameOrAddress::Address(alloy_to_ethers_addr(
            to,
        )));
    }
    if let Some(gas) = unsigned.gas {
        tx.set_gas(U256::from(gas));
    }
    if let Some(value) = unsigned.value {
        tx.set_value(ethers::types::U256::from_dec_str(&value.to_string()).unwrap());
    }
    if let Some(data) = unsigned.data {
        tx.set_data(EthersBytes::from(data.as_ref().to_vec()));
    }
    tx
}

fn sign_typed(
    wallet: &LocalWallet,
    typed: &ethers::types::transaction::eip2718::TypedTransaction,
) -> eyre::Result<TransactionSigned> {
    let sig = futures::executor::block_on(wallet.sign_transaction(typed))?;
    let raw = typed.rlp_signed(&sig);
    let mut slice = raw.as_ref();
    Ok(TransactionSigned::decode(&mut slice).map_err(|e| eyre::eyre!("decode failed: {}", e))?)
}

fn alloy_to_ethers_addr(a: AlloyAddress) -> ethers::types::Address {
    ethers::types::Address::from_slice(a.as_slice())
}
fn ethers_to_alloy_addr(a: ethers::types::Address) -> AlloyAddress {
    AlloyAddress::from_slice(a.as_bytes())
}
fn ethers_to_alloy_u256(u: ethers::types::U256) -> AlloyU256 {
    AlloyU256::from_str_radix(&u.to_string(), 10).unwrap_or_default()
}
