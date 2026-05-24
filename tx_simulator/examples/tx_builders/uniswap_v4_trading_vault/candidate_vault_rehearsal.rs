use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall, SolValue};
use clap::Parser;
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde_json::{json, Value};
use tx_simulator::{
    tx_builders::{
        permit2::build_permit2_approve_tx,
        uniswap_v4::{
            build_token_approval_tx, build_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens,
            build_uniswap_v4_trading_vault_emergency_sell_v4_exact_tokens_for_eth,
            build_universal_router_v4_exact_input_single_tx, UniswapV4PoolKey,
            UniversalRouterV4ExactInputSingleRequest, UniversalRouterV4InputPayment,
        },
    },
    FullSimulationResult, SimulationResult, TxSimulator, UnsignedTransaction,
    UnsignedTxChainSimulation,
};

const DEFAULT_MAX_FEE_PER_GAS_WEI: &str = "50000000000";
const DEFAULT_PRIORITY_FEE_PER_GAS_WEI: &str = "2000000000";
const DEFAULT_SYNTHETIC_ETH_BALANCE_WEI: &str = "100000000000000000000";
const MAX_UINT48: u64 = (1u64 << 48) - 1;

sol! {
    function balanceOf(address owner) external view returns (uint256);
}

#[derive(Debug, Parser)]
#[command(about = "Deploy and rehearse the candidate Uniswap V4 trading vault in tx_simulator")]
struct Args {
    #[arg(long)]
    fixture: Option<PathBuf>,
    #[arg(long)]
    artifact: Option<PathBuf>,
    #[arg(long)]
    reth_datadir: Option<String>,
    #[arg(long)]
    block: Option<u64>,
    #[arg(long)]
    owner: Option<String>,
    #[arg(long)]
    treasury: Option<String>,
    #[arg(long, default_value_t = false)]
    hooks_allowed: bool,
    #[arg(long)]
    amount_in_wei: Option<String>,
    #[arg(long)]
    min_amount_out_raw: Option<String>,
    #[arg(long)]
    sell_min_eth_out_wei: Option<String>,
    #[arg(long, default_value = DEFAULT_SYNTHETIC_ETH_BALANCE_WEI)]
    synthetic_eth_balance_wei: String,
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

#[derive(Debug, Deserialize)]
struct ContractArtifact {
    bytecode: ArtifactBytecode,
}

#[derive(Debug, Deserialize)]
struct ArtifactBytecode {
    object: String,
}

#[derive(Debug, Clone)]
struct ResolvedRoute {
    owner: Address,
    treasury: Address,
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
    owner_eth: U256,
    treasury_eth: U256,
    vault_eth: Option<U256>,
    owner_token: U256,
    vault_token: Option<U256>,
}

impl BalanceSnapshot {
    fn to_json(&self, token_symbol: &str) -> Value {
        json!({
            "owner_eth_wei": self.owner_eth.to_string(),
            "treasury_eth_wei": self.treasury_eth.to_string(),
            "vault_eth_wei": self.vault_eth.map(|value| value.to_string()),
            "token_symbol": token_symbol,
            "owner_token_raw": self.owner_token.to_string(),
            "vault_token_raw": self.vault_token.map(|value| value.to_string())
        })
    }
}

#[derive(Debug)]
struct DirectReport {
    balances_before: BalanceSnapshot,
    buy_result: FullSimulationResult,
    token_bought: U256,
    erc20_approve_result: Option<SimulationResult>,
    permit2_approve_result: Option<SimulationResult>,
    sell_result: Option<FullSimulationResult>,
    eth_received_after_sell: U256,
    balances_after: BalanceSnapshot,
    transactions_executed: usize,
    total_gas_used: u64,
}

#[derive(Debug)]
struct VaultReport {
    vault_address: Address,
    balances_before: BalanceSnapshot,
    deploy_result: FullSimulationResult,
    deploy_code_available: bool,
    buy_result: Option<FullSimulationResult>,
    token_bought: U256,
    sell_result: Option<FullSimulationResult>,
    eth_received_after_sell: U256,
    balances_after: BalanceSnapshot,
    transactions_executed: usize,
    total_gas_used: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let fixture_path = args.fixture.clone().unwrap_or_else(default_fixture_path);
    let artifact_path = args.artifact.clone().unwrap_or_else(default_artifact_path);
    let fixture = load_fixture(&fixture_path)?;
    if fixture.schema != "uniswap_v4_route_fixture_v1" {
        return Err(eyre!(
            "unsupported fixture schema {:?}; expected uniswap_v4_route_fixture_v1",
            fixture.schema
        ));
    }
    if fixture.chain_id != 1 {
        return Err(eyre!(
            "fixture chain_id {} is not mainnet",
            fixture.chain_id
        ));
    }

    let route = resolve_route(&fixture, &args)?;
    if route.token_in != Address::ZERO {
        return Err(eyre!(
            "candidate rehearsal currently supports native ETH input only; got {}",
            route.token_in
        ));
    }
    if route.pool_key.hooks != Address::ZERO && !args.hooks_allowed {
        return Err(eyre!(
            "fixture uses hook {} but --hooks-allowed was not supplied",
            route.pool_key.hooks
        ));
    }

    let vault_init_code = load_vault_init_code(
        &artifact_path,
        route.owner,
        route.treasury,
        route.universal_router,
        route.permit2,
        args.hooks_allowed,
    )?;
    let max_fee_per_gas = parse_u128(&args.max_fee_per_gas_wei, "max_fee_per_gas_wei")?;
    let priority_fee_per_gas =
        parse_u128(&args.priority_fee_per_gas_wei, "priority_fee_per_gas_wei")?;
    let synthetic_eth_balance =
        parse_u256(&args.synthetic_eth_balance_wei, "synthetic_eth_balance_wei")?;

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

    let direct = run_direct_rehearsal(
        &simulator,
        block,
        &route,
        synthetic_eth_balance,
        max_fee_per_gas,
        priority_fee_per_gas,
        args.trace_failure,
    )
    .await?;
    let vault = run_vault_rehearsal(
        &simulator,
        block,
        &route,
        &vault_init_code,
        synthetic_eth_balance,
        max_fee_per_gas,
        priority_fee_per_gas,
        args.trace_failure,
    )
    .await?;

    let report = json!({
        "schema": "uniswap_v4_candidate_vault_rehearsal_v1",
        "fixture_id": fixture.id,
        "fixture_path": fixture_path.display().to_string(),
        "fixture_description": fixture.description,
        "artifact_path": artifact_path.display().to_string(),
        "reth_datadir": reth_datadir,
        "chain_id": fixture.chain_id,
        "latest_context_block": latest_context_block,
        "block": block,
        "owner": route.owner.to_string(),
        "treasury": route.treasury.to_string(),
        "universal_router": route.universal_router.to_string(),
        "permit2": route.permit2.to_string(),
        "route": route_json(&fixture, &route),
        "gas_policy": {
            "kind": "eip1559_public_priority_fee",
            "max_fee_per_gas_wei": max_fee_per_gas.to_string(),
            "priority_fee_per_gas_wei": priority_fee_per_gas.to_string()
        },
        "direct_universal_router": direct_json(&direct, &fixture),
        "candidate_vault": vault_json(&vault, &fixture),
        "comparison": comparison_json(&direct, &vault)
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn run_direct_rehearsal(
    simulator: &TxSimulator,
    block: u64,
    route: &ResolvedRoute,
    synthetic_eth_balance: U256,
    max_fee_per_gas: u128,
    priority_fee_per_gas: u128,
    trace_failure: bool,
) -> Result<DirectReport> {
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    prepare_owner_account(&mut chain, route.owner, synthetic_eth_balance)?;
    let base_fee_per_gas = chain.block_base_fee();
    let balances_before = balances(
        &mut chain,
        route.token_out,
        route.owner,
        route.treasury,
        None,
    )?;

    let mut buy_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: route.universal_router,
            caller: route.owner,
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
    let buy_result = chain.step_with_trace(buy_tx).await?;
    maybe_print_trace("direct buy", &buy_result, trace_failure);
    let token_after_buy = erc20_balance(&mut chain, route.token_out, route.owner)?;
    let token_bought = token_after_buy.saturating_sub(balances_before.owner_token);

    let mut erc20_approve_result = None;
    let mut permit2_approve_result = None;
    let mut sell_result = None;
    let mut eth_received_after_sell = U256::ZERO;

    if buy_result.success && !token_bought.is_zero() {
        let mut erc20_approve =
            build_token_approval_tx(route.owner, route.token_out, route.permit2, U256::MAX);
        apply_eip1559_gas_policy(&mut erc20_approve, max_fee_per_gas, priority_fee_per_gas);
        erc20_approve_result = Some(chain.step(erc20_approve).await?);
    }

    if erc20_approve_result
        .as_ref()
        .map(|result| result.success)
        .unwrap_or(false)
    {
        let mut permit2_approve = build_permit2_approve_tx(
            route.owner,
            route.permit2,
            route.token_out,
            route.universal_router,
            token_bought,
            MAX_UINT48,
        )?;
        apply_eip1559_gas_policy(&mut permit2_approve, max_fee_per_gas, priority_fee_per_gas);
        permit2_approve_result = Some(chain.step(permit2_approve).await?);
    }

    if permit2_approve_result
        .as_ref()
        .map(|result| result.success)
        .unwrap_or(false)
    {
        let eth_before_sell = chain.eth_balance(route.owner)?;
        let mut sell_tx = build_universal_router_v4_exact_input_single_tx(
            &UniversalRouterV4ExactInputSingleRequest {
                universal_router: route.universal_router,
                caller: route.owner,
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
        let result = chain.step_with_trace(sell_tx).await?;
        maybe_print_trace("direct sell", &result, trace_failure);
        let eth_after_sell = chain.eth_balance(route.owner)?;
        eth_received_after_sell = eth_after_sell
            .checked_add(tx_cost_wei(result.gas_used, result.effective_gas_price))
            .and_then(|value| value.checked_sub(eth_before_sell))
            .unwrap_or(U256::ZERO);
        sell_result = Some(result);
    }

    let balances_after = balances(
        &mut chain,
        route.token_out,
        route.owner,
        route.treasury,
        None,
    )?;
    let state = chain.current_state();
    let _ = base_fee_per_gas;
    Ok(DirectReport {
        balances_before,
        buy_result,
        token_bought,
        erc20_approve_result,
        permit2_approve_result,
        sell_result,
        eth_received_after_sell,
        balances_after,
        transactions_executed: state.transaction_count,
        total_gas_used: state.total_gas_used,
    })
}

async fn run_vault_rehearsal(
    simulator: &TxSimulator,
    block: u64,
    route: &ResolvedRoute,
    vault_init_code: &[u8],
    synthetic_eth_balance: U256,
    max_fee_per_gas: u128,
    priority_fee_per_gas: u128,
    trace_failure: bool,
) -> Result<VaultReport> {
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    prepare_owner_account(&mut chain, route.owner, synthetic_eth_balance)?;
    let deploy_nonce = chain.account_nonce(route.owner)?;
    let vault_address = route.owner.create(deploy_nonce);
    let balances_before = balances(
        &mut chain,
        route.token_out,
        route.owner,
        route.treasury,
        Some(vault_address),
    )?;

    let mut deploy_tx = UnsignedTransaction {
        from: Some(route.owner),
        to: None,
        gas: Some(2_000_000),
        value: Some(U256::ZERO),
        data: Some(Bytes::copy_from_slice(vault_init_code)),
        ..Default::default()
    };
    apply_eip1559_gas_policy(&mut deploy_tx, max_fee_per_gas, priority_fee_per_gas);
    let deploy_result = chain.step_with_trace(deploy_tx).await?;
    maybe_print_trace("vault deploy", &deploy_result, trace_failure);
    let deploy_code_available = chain.account_has_code(vault_address)?;

    let mut buy_result = None;
    let mut sell_result = None;
    let mut token_bought = U256::ZERO;
    let mut eth_received_after_sell = U256::ZERO;

    if deploy_result.success && deploy_code_available {
        let vault_token_before = erc20_balance(&mut chain, route.token_out, vault_address)?;
        let mut buy_tx = build_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens(
            vault_address,
            route.owner,
            &route.pool_key,
            route.token_out,
            route.amount_in,
            route.min_amount_out,
            route.deadline,
            &route.hook_data,
        )?;
        apply_eip1559_gas_policy(&mut buy_tx, max_fee_per_gas, priority_fee_per_gas);
        let result = chain.step_with_trace(buy_tx).await?;
        maybe_print_trace("vault buy", &result, trace_failure);
        let vault_token_after = erc20_balance(&mut chain, route.token_out, vault_address)?;
        token_bought = vault_token_after.saturating_sub(vault_token_before);
        buy_result = Some(result);
    }

    if buy_result
        .as_ref()
        .map(|result| result.success)
        .unwrap_or(false)
        && !token_bought.is_zero()
    {
        let owner_eth_before_sell = chain.eth_balance(route.owner)?;
        let mut sell_tx = build_uniswap_v4_trading_vault_emergency_sell_v4_exact_tokens_for_eth(
            vault_address,
            route.owner,
            &route.pool_key,
            route.token_out,
            token_bought,
            route.sell_min_eth_out,
            route.deadline,
            &route.hook_data,
        )?;
        apply_eip1559_gas_policy(&mut sell_tx, max_fee_per_gas, priority_fee_per_gas);
        let result = chain.step_with_trace(sell_tx).await?;
        maybe_print_trace("vault sell", &result, trace_failure);
        let owner_eth_after_sell = chain.eth_balance(route.owner)?;
        eth_received_after_sell = owner_eth_after_sell
            .checked_add(tx_cost_wei(result.gas_used, result.effective_gas_price))
            .and_then(|value| value.checked_sub(owner_eth_before_sell))
            .unwrap_or(U256::ZERO);
        sell_result = Some(result);
    }

    let balances_after = balances(
        &mut chain,
        route.token_out,
        route.owner,
        route.treasury,
        Some(vault_address),
    )?;
    let state = chain.current_state();
    Ok(VaultReport {
        vault_address,
        balances_before,
        deploy_result,
        deploy_code_available,
        buy_result,
        token_bought,
        sell_result,
        eth_received_after_sell,
        balances_after,
        transactions_executed: state.transaction_count,
        total_gas_used: state.total_gas_used,
    })
}

fn direct_json(report: &DirectReport, fixture: &RouteFixture) -> Value {
    json!({
        "balances_before": report.balances_before.to_json(&fixture.token_out_symbol),
        "buy": {
            "transaction": tx_report_full(&report.buy_result),
            "token_out_delta_raw": report.token_bought.to_string()
        },
        "approvals": {
            "erc20_to_permit2": report.erc20_approve_result.as_ref().map(tx_report_basic),
            "permit2_to_universal_router": report.permit2_approve_result.as_ref().map(tx_report_basic)
        },
        "sell": report.sell_result.as_ref().map(|result| json!({
            "transaction": tx_report_full(result),
            "eth_received_wei_before_gas": report.eth_received_after_sell.to_string()
        })),
        "balances_after": report.balances_after.to_json(&fixture.token_out_symbol),
        "totals": {
            "transactions_executed": report.transactions_executed,
            "total_gas_used": report.total_gas_used,
            "direct_router_gas_used": direct_router_gas_used(report),
            "allowance_setup_gas_used": direct_allowance_gas_used(report)
        }
    })
}

fn vault_json(report: &VaultReport, fixture: &RouteFixture) -> Value {
    json!({
        "vault_address": report.vault_address.to_string(),
        "balances_before": report.balances_before.to_json(&fixture.token_out_symbol),
        "deploy": {
            "transaction": tx_report_full(&report.deploy_result),
            "code_available_after_deploy": report.deploy_code_available
        },
        "buy": report.buy_result.as_ref().map(|result| json!({
            "transaction": tx_report_full(result),
            "token_out_delta_raw": report.token_bought.to_string()
        })),
        "sell": report.sell_result.as_ref().map(|result| json!({
            "transaction": tx_report_full(result),
            "eth_received_wei_before_gas": report.eth_received_after_sell.to_string()
        })),
        "balances_after": report.balances_after.to_json(&fixture.token_out_symbol),
        "totals": {
            "transactions_executed": report.transactions_executed,
            "total_gas_used": report.total_gas_used,
            "vault_route_gas_used": vault_route_gas_used(report),
            "deploy_gas_used": report.deploy_result.gas_used
        }
    })
}

fn comparison_json(direct: &DirectReport, vault: &VaultReport) -> Value {
    let direct_router = direct_router_gas_used(direct);
    let direct_full = direct.total_gas_used;
    let vault_route = vault_route_gas_used(vault);
    json!({
        "direct_router_buy_plus_sell_gas": direct_router,
        "direct_full_with_allowances_gas": direct_full,
        "vault_buy_plus_sell_gas": vault_route,
        "vault_route_minus_direct_router_gas": i128::from(vault_route) - i128::from(direct_router),
        "vault_route_minus_direct_full_gas": i128::from(vault_route) - i128::from(direct_full),
        "direct_allowance_setup_gas": direct_allowance_gas_used(direct),
        "vault_deploy_gas": vault.deploy_result.gas_used,
        "notes": [
            "direct_router_buy_plus_sell excludes ERC20 and Permit2 approval transactions",
            "direct_full_with_allowances includes direct buy, ERC20 approve, Permit2 approve, and direct sell",
            "vault_buy_plus_sell excludes candidate deployment; sell includes exact ERC20 and Permit2 allowance lifecycle inside the vault"
        ]
    })
}

fn direct_router_gas_used(report: &DirectReport) -> u64 {
    report.buy_result.gas_used
        + report
            .sell_result
            .as_ref()
            .map(|result| result.gas_used)
            .unwrap_or_default()
}

fn direct_allowance_gas_used(report: &DirectReport) -> u64 {
    report
        .erc20_approve_result
        .as_ref()
        .map(|result| result.gas_used)
        .unwrap_or_default()
        + report
            .permit2_approve_result
            .as_ref()
            .map(|result| result.gas_used)
            .unwrap_or_default()
}

fn vault_route_gas_used(report: &VaultReport) -> u64 {
    report
        .buy_result
        .as_ref()
        .map(|result| result.gas_used)
        .unwrap_or_default()
        + report
            .sell_result
            .as_ref()
            .map(|result| result.gas_used)
            .unwrap_or_default()
}

fn tx_report_full(result: &FullSimulationResult) -> Value {
    let basic = tx_report_fields(
        result.success,
        result.gas_used,
        result.effective_gas_price,
        result.tx_type,
        result.revert_reason.clone(),
    );
    json!({
        "success": basic["success"],
        "gas_used": basic["gas_used"],
        "tx_type": basic["tx_type"],
        "logs": result.logs.len(),
        "revert_reason": basic["revert_reason"],
        "effective_gas_price_wei": basic["effective_gas_price_wei"],
        "estimated_total_fee_spend_wei": basic["estimated_total_fee_spend_wei"]
    })
}

fn tx_report_basic(result: &SimulationResult) -> Value {
    tx_report_fields(
        result.success,
        result.gas_used,
        result.effective_gas_price,
        result.tx_type,
        result.revert_reason.clone(),
    )
}

fn tx_report_fields(
    success: bool,
    gas_used: u64,
    effective_gas_price: Option<u128>,
    tx_type: Option<u8>,
    revert_reason: Option<String>,
) -> Value {
    json!({
        "success": success,
        "gas_used": gas_used,
        "tx_type": tx_type,
        "revert_reason": revert_reason,
        "effective_gas_price_wei": effective_gas_price.map(|value| value.to_string()),
        "estimated_total_fee_spend_wei": tx_cost_wei(gas_used, effective_gas_price).to_string()
    })
}

fn route_json(fixture: &RouteFixture, route: &ResolvedRoute) -> Value {
    json!({
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
    })
}

fn balances(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
    treasury: Address,
    vault: Option<Address>,
) -> Result<BalanceSnapshot> {
    Ok(BalanceSnapshot {
        owner_eth: chain.eth_balance(owner)?,
        treasury_eth: chain.eth_balance(treasury)?,
        vault_eth: vault
            .map(|address| chain.eth_balance(address))
            .transpose()?,
        owner_token: erc20_balance(chain, token, owner)?,
        vault_token: vault
            .map(|address| erc20_balance(chain, token, address))
            .transpose()?,
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

fn prepare_owner_account(
    chain: &mut UnsignedTxChainSimulation,
    owner: Address,
    balance: U256,
) -> Result<()> {
    chain.set_account_as_eoa_for_replay(owner)?;
    chain.set_eth_balance(owner, balance)?;
    Ok(())
}

fn load_vault_init_code(
    artifact_path: &Path,
    owner: Address,
    treasury: Address,
    universal_router: Address,
    permit2: Address,
    hooks_allowed: bool,
) -> Result<Vec<u8>> {
    let raw = fs::read_to_string(artifact_path)
        .wrap_err_with(|| format!("failed to read artifact {}", artifact_path.display()))?;
    let artifact: ContractArtifact = serde_json::from_str(&raw)
        .wrap_err_with(|| format!("failed to parse artifact {}", artifact_path.display()))?;
    let object = artifact.bytecode.object.trim();
    let object = object.strip_prefix("0x").unwrap_or(object);
    if object.is_empty() {
        return Err(eyre!(
            "artifact {} has empty bytecode; run forge build",
            artifact_path.display()
        ));
    }
    let mut init_code = hex::decode(object)
        .wrap_err_with(|| format!("invalid artifact bytecode in {}", artifact_path.display()))?;
    init_code.extend_from_slice(
        &(owner, treasury, universal_router, permit2, hooks_allowed).abi_encode_params(),
    );
    Ok(init_code)
}

fn load_fixture(path: &Path) -> Result<RouteFixture> {
    let raw = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read route fixture {}", path.display()))?;
    serde_json::from_str(&raw)
        .wrap_err_with(|| format!("failed to parse route fixture {}", path.display()))
}

fn resolve_route(fixture: &RouteFixture, args: &Args) -> Result<ResolvedRoute> {
    let owner = parse_address(args.owner.as_deref().unwrap_or(&fixture.caller), "owner")?;
    let treasury = args
        .treasury
        .as_deref()
        .map(|value| parse_address(value, "treasury"))
        .transpose()?
        .unwrap_or(owner);
    Ok(ResolvedRoute {
        owner,
        treasury,
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

fn default_fixture_path() -> PathBuf {
    eth_workspace_root().join(
        "deploy/onchain/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json",
    )
}

fn default_artifact_path() -> PathBuf {
    eth_workspace_root()
        .join("solidity/baygus-executor/out/UniswapV4TradingVault.sol/UniswapV4TradingVault.json")
}

fn eth_workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tx_simulator manifest should live below ETH workspace root")
        .to_path_buf()
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

fn tx_cost_wei(gas_used: u64, effective_gas_price: Option<u128>) -> U256 {
    U256::from(gas_used) * U256::from(effective_gas_price.unwrap_or_default())
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
