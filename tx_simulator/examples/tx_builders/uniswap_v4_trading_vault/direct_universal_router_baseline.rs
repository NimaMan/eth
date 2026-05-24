use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};
use clap::Parser;
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde_json::{json, Value};
use tx_simulator::{
    tx_builders::{
        permit2::build_permit2_approve_tx,
        uniswap_v4::{
            build_token_approval_tx, build_universal_router_v4_exact_input_single_tx,
            UniswapV4PoolKey, UniversalRouterV4ExactInputSingleRequest,
            UniversalRouterV4InputPayment,
        },
    },
    FullSimulationResult, SimulationResult, TxSimulator, UnsignedTransaction,
    UnsignedTxChainSimulation,
};

const DEFAULT_MAX_FEE_PER_GAS_WEI: &str = "50000000000";
const DEFAULT_PRIORITY_FEE_PER_GAS_WEI: &str = "2000000000";
const MAX_UINT48: u64 = (1u64 << 48) - 1;

sol! {
    function balanceOf(address owner) external view returns (uint256);
}

#[derive(Debug, Parser)]
#[command(
    about = "Simulate the direct Uniswap V4 Universal Router route that the candidate V4 trading vault must beat"
)]
struct Args {
    #[arg(long)]
    fixture: Option<PathBuf>,
    #[arg(long)]
    reth_datadir: Option<String>,
    #[arg(long)]
    block: Option<u64>,
    #[arg(long)]
    caller: Option<String>,
    #[arg(long)]
    amount_in_wei: Option<String>,
    #[arg(long)]
    min_amount_out_raw: Option<String>,
    #[arg(long)]
    sell_min_eth_out_wei: Option<String>,
    #[arg(long, default_value = DEFAULT_MAX_FEE_PER_GAS_WEI)]
    max_fee_per_gas_wei: String,
    #[arg(long, default_value = DEFAULT_PRIORITY_FEE_PER_GAS_WEI)]
    priority_fee_per_gas_wei: String,
    #[arg(long, default_value_t = false)]
    trace_failure: bool,
}

#[derive(Debug, Deserialize)]
struct RouteFixture {
    schema: String,
    id: String,
    description: String,
    chain_id: u64,
    preferred_block: Option<u64>,
    caller: String,
    universal_router: String,
    permit2: String,
    token_in: String,
    token_in_symbol: String,
    token_in_decimals: u8,
    token_out: String,
    token_out_symbol: String,
    token_out_decimals: u8,
    amount_in_wei: String,
    min_amount_out_raw: String,
    sell_min_eth_out_wei: String,
    deadline_raw: String,
    #[serde(default)]
    hook_data_hex: String,
    hook_policy: String,
    pool_key: FixturePoolKey,
}

#[derive(Debug, Deserialize)]
struct FixturePoolKey {
    currency0: String,
    currency1: String,
    fee: u32,
    tick_spacing: i32,
    hooks: String,
}

#[derive(Debug, Clone)]
struct ResolvedRoute {
    caller: Address,
    universal_router: Address,
    permit2: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    min_amount_out: U256,
    sell_min_eth_out: U256,
    deadline: U256,
    hook_data: Vec<u8>,
    pool_key: UniswapV4PoolKey,
}

#[derive(Debug, Clone)]
struct BalanceSnapshot {
    caller_eth: U256,
    token_out: U256,
}

impl BalanceSnapshot {
    fn to_json(&self, token_symbol: &str) -> Value {
        json!({
            "caller_eth_wei": self.caller_eth.to_string(),
            "token_out_symbol": token_symbol,
            "token_out_raw": self.token_out.to_string()
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let fixture_path = args.fixture.clone().unwrap_or_else(default_fixture_path);
    let fixture = load_fixture(&fixture_path)?;
    if fixture.schema != "uniswap_v4_route_fixture_v1" {
        return Err(eyre!(
            "unsupported fixture schema {:?}; expected uniswap_v4_route_fixture_v1",
            fixture.schema
        ));
    }

    let route = resolve_route(&fixture, &args)?;
    if route.token_in != Address::ZERO {
        return Err(eyre!(
            "this baseline currently supports native ETH input only; got {}",
            route.token_in
        ));
    }
    if fixture.chain_id != 1 {
        return Err(eyre!(
            "fixture chain_id {} is not mainnet chain_id 1",
            fixture.chain_id
        ));
    }

    let max_fee_per_gas = parse_u128(&args.max_fee_per_gas_wei, "max_fee_per_gas_wei")?;
    let priority_fee_per_gas =
        parse_u128(&args.priority_fee_per_gas_wei, "priority_fee_per_gas_wei")?;
    let reth_datadir = match args.reth_datadir {
        Some(path) => path,
        None => match std::env::var("RETH_DATADIR") {
            Ok(path) => path,
            Err(_) => tx_simulator::config::repo::reth_datadir()?,
        },
    };

    let simulator = TxSimulator::new(&reth_datadir)?;
    let latest_context_block = simulator.latest_historical_context_block_number()?;
    let block = args
        .block
        .or(fixture.preferred_block)
        .unwrap_or(latest_context_block);
    if block > latest_context_block {
        return Err(eyre!(
            "requested block {block} is ahead of latest local simulation context {latest_context_block}"
        ));
    }

    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee_per_gas = chain.block_base_fee();
    let balances_before = balances(&mut chain, route.token_out, route.caller)?;

    let mut buy_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: route.universal_router,
            caller: route.caller,
            pool_key: route.pool_key.clone(),
            token_in: route.token_in,
            token_out: route.token_out,
            amount_in: route.amount_in,
            min_amount_out: route.min_amount_out,
            deadline: route.deadline,
            hook_data: route.hook_data.clone(),
            input_payment: UniversalRouterV4InputPayment::NativeEth,
        },
    )?;
    apply_eip1559_gas_policy(&mut buy_tx, max_fee_per_gas, priority_fee_per_gas);
    let buy_result = chain
        .step_with_trace(buy_tx)
        .await
        .wrap_err("direct V4 Universal Router buy simulation failed")?;
    maybe_print_trace("direct V4 buy", &buy_result, args.trace_failure);

    let token_after_buy = erc20_balance(&mut chain, route.token_out, route.caller)?;
    let token_bought = token_after_buy.saturating_sub(balances_before.token_out);

    let mut erc20_approve_result = None;
    let mut permit2_approve_result = None;
    let mut sell_result = None;
    let mut eth_received_after_sell = U256::ZERO;

    if buy_result.success && !token_bought.is_zero() {
        let mut erc20_approve =
            build_token_approval_tx(route.caller, route.token_out, route.permit2, U256::MAX);
        apply_eip1559_gas_policy(&mut erc20_approve, max_fee_per_gas, priority_fee_per_gas);
        let result = chain
            .step(erc20_approve)
            .await
            .wrap_err("ERC20 approve Permit2 simulation failed")?;
        erc20_approve_result = Some(result);
    }

    if erc20_approve_result
        .as_ref()
        .map(|result| result.success)
        .unwrap_or(false)
    {
        let mut permit2_approve = build_permit2_approve_tx(
            route.caller,
            route.permit2,
            route.token_out,
            route.universal_router,
            token_bought,
            MAX_UINT48,
        )?;
        apply_eip1559_gas_policy(&mut permit2_approve, max_fee_per_gas, priority_fee_per_gas);
        let result = chain
            .step(permit2_approve)
            .await
            .wrap_err("Permit2 approve Universal Router simulation failed")?;
        permit2_approve_result = Some(result);
    }

    if permit2_approve_result
        .as_ref()
        .map(|result| result.success)
        .unwrap_or(false)
    {
        let eth_before_sell = chain.eth_balance(route.caller)?;
        let mut sell_tx = build_universal_router_v4_exact_input_single_tx(
            &UniversalRouterV4ExactInputSingleRequest {
                universal_router: route.universal_router,
                caller: route.caller,
                pool_key: route.pool_key.clone(),
                token_in: route.token_out,
                token_out: route.token_in,
                amount_in: token_bought,
                min_amount_out: route.sell_min_eth_out,
                deadline: route.deadline,
                hook_data: route.hook_data.clone(),
                input_payment: UniversalRouterV4InputPayment::Permit2User,
            },
        )?;
        apply_eip1559_gas_policy(&mut sell_tx, max_fee_per_gas, priority_fee_per_gas);
        let result = chain
            .step_with_trace(sell_tx)
            .await
            .wrap_err("direct V4 Universal Router sell simulation failed")?;
        maybe_print_trace("direct V4 sell", &result, args.trace_failure);

        let eth_after_sell = chain.eth_balance(route.caller)?;
        let sell_gas_cost = tx_cost_wei(result.gas_used, result.effective_gas_price);
        eth_received_after_sell = eth_after_sell
            .checked_add(sell_gas_cost)
            .and_then(|value| value.checked_sub(eth_before_sell))
            .unwrap_or(U256::ZERO);
        sell_result = Some(result);
    }

    let balances_after = balances(&mut chain, route.token_out, route.caller)?;
    let state = chain.current_state();
    let direct_router_gas_used = buy_result.gas_used
        + sell_result
            .as_ref()
            .map(|result| result.gas_used)
            .unwrap_or_default();

    let report = json!({
        "schema": "uniswap_v4_direct_universal_router_baseline_v1",
        "fixture_id": fixture.id,
        "fixture_path": fixture_path.display().to_string(),
        "fixture_description": fixture.description,
        "reth_datadir": reth_datadir,
        "chain_id": fixture.chain_id,
        "latest_context_block": latest_context_block,
        "block": block,
        "caller": route.caller.to_string(),
        "universal_router": route.universal_router.to_string(),
        "permit2": route.permit2.to_string(),
        "route": {
            "token_in": route.token_in.to_string(),
            "token_in_symbol": fixture.token_in_symbol,
            "token_in_decimals": fixture.token_in_decimals,
            "token_out": route.token_out.to_string(),
            "token_out_symbol": fixture.token_out_symbol,
            "token_out_decimals": fixture.token_out_decimals,
            "amount_in_wei": route.amount_in.to_string(),
            "min_amount_out_raw": route.min_amount_out.to_string(),
            "sell_min_eth_out_wei": route.sell_min_eth_out.to_string(),
            "deadline_raw": route.deadline.to_string(),
            "hook_data_hex": format!("0x{}", hex::encode(&route.hook_data)),
            "hook_policy": fixture.hook_policy,
            "pool_key": {
                "currency0": route.pool_key.currency0.to_string(),
                "currency1": route.pool_key.currency1.to_string(),
                "fee": route.pool_key.fee,
                "tick_spacing": route.pool_key.tick_spacing,
                "hooks": route.pool_key.hooks.to_string()
            }
        },
        "gas_policy": {
            "kind": "eip1559_public_priority_fee",
            "max_fee_per_gas_wei": max_fee_per_gas.to_string(),
            "priority_fee_per_gas_wei": priority_fee_per_gas.to_string(),
            "block_base_fee_per_gas_wei": base_fee_per_gas.map(|value| value.to_string())
        },
        "balances_before": balances_before.to_json(&fixture.token_out_symbol),
        "buy": {
            "transaction": tx_report_full(&buy_result, base_fee_per_gas),
            "token_out_delta_raw": token_bought.to_string()
        },
        "approvals": {
            "erc20_to_permit2": erc20_approve_result
                .as_ref()
                .map(|result| tx_report_basic(result, base_fee_per_gas)),
            "permit2_to_universal_router": permit2_approve_result
                .as_ref()
                .map(|result| tx_report_basic(result, base_fee_per_gas))
        },
        "sell": sell_result.as_ref().map(|result| json!({
            "transaction": tx_report_full(result, base_fee_per_gas),
            "eth_received_wei_before_gas": eth_received_after_sell.to_string()
        })),
        "balances_after": balances_after.to_json(&fixture.token_out_symbol),
        "totals": {
            "transactions_executed": state.transaction_count,
            "total_gas_used": state.total_gas_used,
            "direct_router_gas_used": direct_router_gas_used,
            "allowance_setup_gas_used": erc20_approve_result.as_ref().map(|result| result.gas_used).unwrap_or_default()
                + permit2_approve_result.as_ref().map(|result| result.gas_used).unwrap_or_default()
        }
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn default_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tx_simulator manifest should live below ETH workspace root")
        .join("deploy/onchain/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json")
}

fn load_fixture(path: &Path) -> Result<RouteFixture> {
    let raw = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read route fixture {}", path.display()))?;
    serde_json::from_str(&raw)
        .wrap_err_with(|| format!("failed to parse route fixture {}", path.display()))
}

fn resolve_route(fixture: &RouteFixture, args: &Args) -> Result<ResolvedRoute> {
    let caller = parse_address(args.caller.as_deref().unwrap_or(&fixture.caller), "caller")?;
    Ok(ResolvedRoute {
        caller,
        universal_router: parse_address(&fixture.universal_router, "universal_router")?,
        permit2: parse_address(&fixture.permit2, "permit2")?,
        token_in: parse_address(&fixture.token_in, "token_in")?,
        token_out: parse_address(&fixture.token_out, "token_out")?,
        amount_in: parse_u256(
            args.amount_in_wei
                .as_deref()
                .unwrap_or(&fixture.amount_in_wei),
            "amount_in_wei",
        )?,
        min_amount_out: parse_u256(
            args.min_amount_out_raw
                .as_deref()
                .unwrap_or(&fixture.min_amount_out_raw),
            "min_amount_out_raw",
        )?,
        sell_min_eth_out: parse_u256(
            args.sell_min_eth_out_wei
                .as_deref()
                .unwrap_or(&fixture.sell_min_eth_out_wei),
            "sell_min_eth_out_wei",
        )?,
        deadline: parse_u256(&fixture.deadline_raw, "deadline_raw")?,
        hook_data: parse_hex_data(&fixture.hook_data_hex, "hook_data_hex")?,
        pool_key: UniswapV4PoolKey {
            currency0: parse_address(&fixture.pool_key.currency0, "pool_key.currency0")?,
            currency1: parse_address(&fixture.pool_key.currency1, "pool_key.currency1")?,
            fee: fixture.pool_key.fee,
            tick_spacing: fixture.pool_key.tick_spacing,
            hooks: parse_address(&fixture.pool_key.hooks, "pool_key.hooks")?,
        },
    })
}

fn balances(
    chain: &mut UnsignedTxChainSimulation,
    token_out: Address,
    caller: Address,
) -> Result<BalanceSnapshot> {
    Ok(BalanceSnapshot {
        caller_eth: chain.eth_balance(caller)?,
        token_out: erc20_balance(chain, token_out, caller)?,
    })
}

fn erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
) -> Result<U256> {
    let result =
        chain.simulate_view_call(token, Bytes::from(balanceOfCall { owner }.abi_encode()))?;
    if !result.success {
        return Err(eyre!("balanceOf({owner}) failed for token {token}"));
    }
    decode_u256(&result.output, "balanceOf output")
}

fn tx_report_full(result: &FullSimulationResult, base_fee_per_gas: Option<u128>) -> Value {
    let common = tx_report_fields(
        result.success,
        result.gas_used,
        result.effective_gas_price,
        result.tx_type,
        result.revert_reason.clone(),
        base_fee_per_gas,
    );
    json!({
        "success": common["success"],
        "gas_used": common["gas_used"],
        "tx_type": common["tx_type"],
        "logs": result.logs.len(),
        "revert_reason": common["revert_reason"],
        "effective_gas_price_wei": common["effective_gas_price_wei"],
        "effective_priority_fee_per_gas_wei": common["effective_priority_fee_per_gas_wei"],
        "estimated_total_fee_spend_wei": common["estimated_total_fee_spend_wei"],
        "estimated_priority_fee_spend_wei": common["estimated_priority_fee_spend_wei"]
    })
}

fn tx_report_basic(result: &SimulationResult, base_fee_per_gas: Option<u128>) -> Value {
    tx_report_fields(
        result.success,
        result.gas_used,
        result.effective_gas_price,
        result.tx_type,
        result.revert_reason.clone(),
        base_fee_per_gas,
    )
}

fn tx_report_fields(
    success: bool,
    gas_used: u64,
    effective_gas_price: Option<u128>,
    tx_type: Option<u8>,
    revert_reason: Option<String>,
    base_fee_per_gas: Option<u128>,
) -> Value {
    let effective_gas_price_value = effective_gas_price.unwrap_or_default();
    let effective_priority_fee = base_fee_per_gas
        .map(|base_fee| effective_gas_price_value.saturating_sub(base_fee))
        .unwrap_or_default();
    let total_fee_spend = tx_cost_wei(gas_used, effective_gas_price);
    let priority_fee_spend = U256::from(gas_used) * U256::from(effective_priority_fee);
    json!({
        "success": success,
        "gas_used": gas_used,
        "tx_type": tx_type,
        "revert_reason": revert_reason,
        "effective_gas_price_wei": effective_gas_price.map(|value| value.to_string()),
        "effective_priority_fee_per_gas_wei": effective_priority_fee.to_string(),
        "estimated_total_fee_spend_wei": total_fee_spend.to_string(),
        "estimated_priority_fee_spend_wei": priority_fee_spend.to_string()
    })
}

fn tx_cost_wei(gas_used: u64, effective_gas_price: Option<u128>) -> U256 {
    U256::from(gas_used) * U256::from(effective_gas_price.unwrap_or_default())
}

fn apply_eip1559_gas_policy(
    tx: &mut UnsignedTransaction,
    max_fee_per_gas: u128,
    priority_fee_per_gas: u128,
) {
    if tx.gas.is_none() {
        tx.gas = Some(2_500_000);
    }
    tx.gas_price = None;
    tx.max_fee_per_gas = Some(max_fee_per_gas);
    tx.max_priority_fee_per_gas = Some(priority_fee_per_gas);
}

fn maybe_print_trace(label: &str, result: &FullSimulationResult, enabled: bool) {
    if enabled && !result.success {
        eprintln!("{label} call trace:\n{:#?}", result.call_trace);
    }
}

fn parse_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value).wrap_err_with(|| format!("invalid {label} address {value:?}"))
}

fn parse_u256(value: &str, label: &str) -> Result<U256> {
    U256::from_str(value).wrap_err_with(|| format!("invalid {label} value {value:?}"))
}

fn parse_u128(value: &str, label: &str) -> Result<u128> {
    value
        .parse::<u128>()
        .wrap_err_with(|| format!("invalid {label} value {value:?}"))
}

fn parse_hex_data(value: &str, label: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    hex::decode(hex).wrap_err_with(|| format!("invalid {label} hex data {value:?}"))
}

fn decode_u256(output: &Bytes, label: &str) -> Result<U256> {
    if output.len() < 32 {
        return Err(eyre!("{label} is shorter than 32 bytes"));
    }
    Ok(U256::from_be_slice(&output[..32]))
}
