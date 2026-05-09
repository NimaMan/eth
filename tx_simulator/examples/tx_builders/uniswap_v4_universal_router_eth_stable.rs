use alloy_primitives::{address, Address, Bytes, U256};
use eyre::{eyre, Result};
use tx_simulator::{
    tx_builders::{
        permit2::build_permit2_approve_tx,
        uniswap_v4::{
            build_token_approval_tx, build_universal_router_v4_exact_input_single_tx,
            UniswapV4PoolKey, UniversalRouterV4ExactInputSingleRequest,
            UniversalRouterV4InputPayment,
        },
    },
    TxSimulator, UnsignedTransaction,
};

const ONE_ETH_WEI: u128 = 1_000_000_000_000_000_000;
const BUYER: Address = address!("95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5");
const USDC: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const UNIVERSAL_ROUTER: Address = address!("66a9893cC07D91D95644AEDD05D03f95e1dBA8Af");
const PERMIT2: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");
const MAX_UINT48: u64 = (1u64 << 48) - 1;
const GAS_PRICE_WEI: u128 = 50_000_000_000;

const ERC20_BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];

#[tokio::main]
async fn main() -> Result<()> {
    let reth_datadir = match std::env::var("RETH_DATADIR") {
        Ok(path) => path,
        Err(_) => tx_simulator::config::repo::reth_datadir()?,
    };
    let simulator = TxSimulator::new(&reth_datadir)?;
    let latest_block = simulator.get_latest_block()?;
    let block_number = selected_block(latest_block)?;
    let amount_in = U256::from(ONE_ETH_WEI);
    let pool_key = native_eth_usdc_pool_key();

    println!("Uniswap v4 Universal Router ETH -> USDC -> ETH simulation");
    println!("=========================================================");
    println!("Reth datadir     : {reth_datadir}");
    println!("Latest block     : {latest_block}");
    println!("Simulation block : {block_number}");
    println!("Caller           : {BUYER}");
    println!("Universal Router : {UNIVERSAL_ROUTER}");
    println!("Permit2          : {PERMIT2}");
    println!("Amount in        : {} ETH", format_units(amount_in, 18, 6));

    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;
    let usdc_before = erc20_balance_of(&mut chain, USDC, BUYER)?;

    let mut buy_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: UNIVERSAL_ROUTER,
            caller: BUYER,
            pool_key: pool_key.clone(),
            token_in: Address::ZERO,
            token_out: USDC,
            amount_in,
            min_amount_out: U256::ZERO,
            deadline: U256::MAX,
            hook_data: Vec::new(),
            input_payment: UniversalRouterV4InputPayment::NativeEth,
        },
    )?;
    apply_simple_gas_policy(&mut buy_tx);
    let buy_result = chain.step_with_trace(buy_tx).await?;
    ensure_full_success("ETH -> USDC buy", &buy_result)?;

    let usdc_after_buy = erc20_balance_of(&mut chain, USDC, BUYER)?;
    let usdc_bought = usdc_after_buy.saturating_sub(usdc_before);
    println!();
    println!("Buy");
    println!("---");
    println!("Success  : {}", buy_result.success);
    println!("Gas used : {}", buy_result.gas_used);
    println!("USDC out : {}", format_units(usdc_bought, 6, 6));

    let mut erc20_approve = build_token_approval_tx(BUYER, USDC, PERMIT2, U256::MAX);
    apply_simple_gas_policy(&mut erc20_approve);
    let erc20_approve_result = chain.step(erc20_approve).await?;
    ensure_basic_success("USDC approve Permit2", &erc20_approve_result)?;

    let mut permit2_approve = build_permit2_approve_tx(
        BUYER,
        PERMIT2,
        USDC,
        UNIVERSAL_ROUTER,
        usdc_bought,
        MAX_UINT48,
    )?;
    apply_simple_gas_policy(&mut permit2_approve);
    let permit2_approve_result = chain.step(permit2_approve).await?;
    ensure_basic_success("Permit2 approve Universal Router", &permit2_approve_result)?;

    let eth_before_sell = chain.eth_balance(BUYER)?;
    let mut sell_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: UNIVERSAL_ROUTER,
            caller: BUYER,
            pool_key,
            token_in: USDC,
            token_out: Address::ZERO,
            amount_in: usdc_bought,
            min_amount_out: U256::ZERO,
            deadline: U256::MAX,
            hook_data: Vec::new(),
            input_payment: UniversalRouterV4InputPayment::Permit2User,
        },
    )?;
    apply_simple_gas_policy(&mut sell_tx);
    let sell_result = chain.step_with_trace(sell_tx).await?;
    ensure_full_success("USDC -> ETH sell", &sell_result)?;
    let eth_after_sell = chain.eth_balance(BUYER)?;
    let sell_gas_cost = U256::from(sell_result.gas_used) * U256::from(GAS_PRICE_WEI);
    let eth_received = eth_after_sell
        .checked_add(sell_gas_cost)
        .and_then(|value| value.checked_sub(eth_before_sell))
        .unwrap_or(U256::ZERO);

    println!();
    println!("Approvals");
    println!("---------");
    println!("ERC20 approve gas   : {}", erc20_approve_result.gas_used);
    println!("Permit2 approve gas : {}", permit2_approve_result.gas_used);

    println!();
    println!("Sell");
    println!("----");
    println!("Success  : {}", sell_result.success);
    println!("Gas used : {}", sell_result.gas_used);
    println!("ETH out  : {}", format_units(eth_received, 18, 9));

    Ok(())
}

fn native_eth_usdc_pool_key() -> UniswapV4PoolKey {
    UniswapV4PoolKey {
        currency0: Address::ZERO,
        currency1: USDC,
        fee: 500,
        tick_spacing: 10,
        hooks: Address::ZERO,
    }
}

fn selected_block(latest_block: u64) -> Result<u64> {
    match std::env::var("UNISWAP_V4_UR_SIM_BLOCK") {
        Ok(value) => {
            let requested = value
                .trim()
                .parse::<u64>()
                .map_err(|err| eyre!("invalid UNISWAP_V4_UR_SIM_BLOCK value {value:?}: {err}"))?;
            if requested > latest_block {
                return Err(eyre!(
                    "UNISWAP_V4_UR_SIM_BLOCK {requested} is ahead of latest local block {latest_block}"
                ));
            }
            Ok(requested)
        }
        Err(std::env::VarError::NotPresent) => Ok(latest_block),
        Err(err) => Err(eyre!("failed to read UNISWAP_V4_UR_SIM_BLOCK: {err}")),
    }
}

fn ensure_basic_success(label: &str, result: &tx_simulator::SimulationResult) -> Result<()> {
    if result.success {
        return Ok(());
    }

    Err(eyre!(
        "{label} failed: {}",
        result
            .revert_reason
            .as_deref()
            .unwrap_or("transaction reverted without decoded reason")
    ))
}

fn ensure_full_success(label: &str, result: &tx_simulator::FullSimulationResult) -> Result<()> {
    if result.success {
        return Ok(());
    }

    if trace_failures_enabled() {
        println!("{:#?}", result.call_trace);
    }

    Err(eyre!(
        "{label} failed: {}",
        result
            .revert_reason
            .as_deref()
            .unwrap_or("transaction reverted without decoded reason")
    ))
}

fn erc20_balance_of(
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
) -> Result<U256> {
    let mut data = Vec::with_capacity(36);
    data.extend_from_slice(&ERC20_BALANCE_OF_SELECTOR);
    data.extend_from_slice(&pad_address(owner));

    let result = chain.simulate_view_call(token, Bytes::from(data))?;
    if !result.success {
        return Err(eyre!("balanceOf({owner}) view call failed for {token}"));
    }
    decode_u256_word(result.output.as_ref(), 0, "balanceOf output")
}

fn decode_u256_word(data: &[u8], offset: usize, label: &str) -> Result<U256> {
    let end = offset + 32;
    let word = data.get(offset..end).ok_or_else(|| {
        eyre!(
            "{label} too short: need word at offset {offset}, got {} bytes",
            data.len()
        )
    })?;
    Ok(U256::from_be_slice(word))
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..32].copy_from_slice(address.as_slice());
    out
}

fn apply_simple_gas_policy(tx: &mut UnsignedTransaction) {
    if tx.gas.is_none() {
        tx.gas = Some(2_500_000);
    }
    if tx.max_fee_per_gas.is_none() && tx.gas_price.is_none() {
        tx.gas_price = Some(GAS_PRICE_WEI);
    }
}

fn trace_failures_enabled() -> bool {
    std::env::var("UNISWAP_V4_UR_TRACE_FAILURE")
        .map(|value| {
            let lowered = value.trim().to_ascii_lowercase();
            lowered == "1" || lowered == "true" || lowered == "yes"
        })
        .unwrap_or(false)
}

fn format_units(amount: U256, decimals: usize, precision: usize) -> String {
    if amount.is_zero() {
        return "0".to_string();
    }

    let digits = amount.to_string();
    if decimals == 0 {
        return digits;
    }

    let padded;
    let value = if digits.len() <= decimals {
        padded = format!("{:0>width$}", digits, width = decimals + 1);
        padded.as_str()
    } else {
        digits.as_str()
    };

    let split = value.len() - decimals;
    let whole = &value[..split];
    let frac = &value[split..];
    let frac = &frac[..frac.len().min(precision)];
    let frac = frac.trim_end_matches('0');

    if frac.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{frac}")
    }
}
