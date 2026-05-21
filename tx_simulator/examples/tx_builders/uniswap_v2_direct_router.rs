use std::str::FromStr;

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};
use clap::{Parser, ValueEnum};
use eyre::{eyre, Result, WrapErr};
use serde_json::{json, Value};
use tx_simulator::{
    tx_builders::uniswap_v2::{
        build_approve_v2, build_buy_swap_v2_supporting_fee_with_min_out,
        build_sell_swap_v2_with_min_out, Router,
    },
    FullSimulationResult, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};

const DEFAULT_OWNER: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const DEFAULT_TOKEN: &str = "0x12512a46d971BD035D9466246bc46DAf389E281c";
const DEFAULT_SYMBOL: &str = "OTTO";
const DEFAULT_SELL_AMOUNT_RAW: &str = "100000000000000000000";
const DEFAULT_BUY_ETH_WEI: &str = "10000000000000000";
const DEFAULT_MIN_OUT: &str = "1";
const DEFAULT_DEADLINE: u64 = 1_800_000_000;
const DEFAULT_MAX_FEE_PER_GAS_WEI: &str = "50000000000";
const DEFAULT_PRIORITY_FEE_PER_GAS_WEI: &str = "2000000000";

sol! {
    function balanceOf(address owner) external view returns (uint256);
}

#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum Scenario {
    WalletApproveThenSell,
    BuyApproveThenSell,
    CurrentStateSell,
}

#[derive(Debug, Parser)]
#[command(about = "Simulate direct Uniswap V2 router buy/sell without the trading vault")]
struct Args {
    #[arg(long, default_value = DEFAULT_OWNER)]
    owner: String,
    #[arg(long, default_value = DEFAULT_ROUTER)]
    router: String,
    #[arg(long, default_value = DEFAULT_TOKEN)]
    token: String,
    #[arg(long, default_value = DEFAULT_SYMBOL)]
    symbol: String,
    #[arg(long, value_enum, default_value_t = Scenario::WalletApproveThenSell)]
    scenario: Scenario,
    #[arg(long, default_value = DEFAULT_SELL_AMOUNT_RAW)]
    sell_amount_raw: String,
    #[arg(long, default_value = DEFAULT_BUY_ETH_WEI)]
    buy_eth_wei: String,
    #[arg(long, default_value = DEFAULT_MIN_OUT)]
    min_tokens_out_raw: String,
    #[arg(long, default_value = DEFAULT_MIN_OUT)]
    min_eth_out_wei: String,
    #[arg(long, default_value_t = DEFAULT_DEADLINE)]
    deadline: u64,
    #[arg(long)]
    block: Option<u64>,
    #[arg(long, default_value = DEFAULT_MAX_FEE_PER_GAS_WEI)]
    max_fee_per_gas_wei: String,
    #[arg(long, default_value = DEFAULT_PRIORITY_FEE_PER_GAS_WEI)]
    priority_fee_per_gas_wei: String,
    #[arg(long, default_value_t = false)]
    trace_failure: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let owner = parse_address(&args.owner, "owner")?;
    let router = parse_address(&args.router, "router")?;
    let token = parse_address(&args.token, "token")?;
    let requested_sell_amount = parse_u256(&args.sell_amount_raw, "sell_amount_raw")?;
    let buy_eth = parse_u256(&args.buy_eth_wei, "buy_eth_wei")?;
    let min_tokens_out = parse_u256(&args.min_tokens_out_raw, "min_tokens_out_raw")?;
    let min_eth_out = parse_u256(&args.min_eth_out_wei, "min_eth_out_wei")?;
    let max_fee_per_gas = parse_u128(&args.max_fee_per_gas_wei, "max_fee_per_gas_wei")?;
    let priority_fee_per_gas =
        parse_u128(&args.priority_fee_per_gas_wei, "priority_fee_per_gas_wei")?;

    let reth_datadir = match std::env::var("RETH_DATADIR") {
        Ok(path) => path,
        Err(_) => tx_simulator::config::repo::reth_datadir()?,
    };
    let simulator = TxSimulator::new(&reth_datadir)?;
    let latest_block = simulator.latest_historical_context_block_number()?;
    let block = args.block.unwrap_or(latest_block);
    if block > latest_block {
        return Err(eyre!(
            "requested block {block} is ahead of latest local simulation context {latest_block}"
        ));
    }

    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee_per_gas = chain.block_base_fee();
    let balances_before = balances(&mut chain, token, owner)?;

    let mut buy_result = None;

    let sell_amount = match args.scenario {
        Scenario::BuyApproveThenSell => {
            let mut buy_tx = build_buy_swap_v2_supporting_fee_with_min_out(
                Router::Custom(router),
                owner,
                token,
                buy_eth,
                min_tokens_out,
                args.deadline,
            );
            apply_eip1559_gas_policy(&mut buy_tx, max_fee_per_gas, priority_fee_per_gas);
            let result = chain
                .step_with_trace(buy_tx)
                .await
                .wrap_err("direct router buy simulation failed")?;
            maybe_print_trace("direct router buy", &result, args.trace_failure);
            buy_result = Some(result);
            let owner_token_after_buy = erc20_balance(&mut chain, token, owner)?;
            owner_token_after_buy.saturating_sub(balances_before.owner_token)
        }
        Scenario::WalletApproveThenSell => requested_sell_amount,
        Scenario::CurrentStateSell => requested_sell_amount.min(balances_before.owner_token),
    };

    if sell_amount.is_zero() {
        return Err(eyre!(
            "no token balance available for direct router sell in {:?}",
            args.scenario
        ));
    }
    if erc20_balance(&mut chain, token, owner)? < sell_amount {
        return Err(eyre!(
            "owner token balance is below requested sell amount {sell_amount}"
        ));
    }

    let mut approve_tx = build_approve_v2(Router::Custom(router), owner, token, sell_amount);
    apply_eip1559_gas_policy(&mut approve_tx, max_fee_per_gas, priority_fee_per_gas);
    let result = chain
        .step_with_trace(approve_tx)
        .await
        .wrap_err("direct router approve simulation failed")?;
    maybe_print_trace("direct router approve", &result, args.trace_failure);
    let approve_result = result;

    let mut sell_tx = build_sell_swap_v2_with_min_out(
        Router::Custom(router),
        owner,
        token,
        sell_amount,
        min_eth_out,
        args.deadline,
    );
    apply_eip1559_gas_policy(&mut sell_tx, max_fee_per_gas, priority_fee_per_gas);
    let result = chain
        .step_with_trace(sell_tx)
        .await
        .wrap_err("direct router sell simulation failed")?;
    maybe_print_trace("direct router sell", &result, args.trace_failure);
    let sell_result = result;

    let balances_after = balances(&mut chain, token, owner)?;
    let state = chain.current_state();
    let report = json!({
        "schema": "uniswap_v2_direct_router_simulation_v1",
        "scenario": format!("{:?}", args.scenario),
        "reth_datadir": reth_datadir,
        "latest_context_block": latest_block,
        "block": block,
        "owner": owner.to_string(),
        "router": router.to_string(),
        "token": token.to_string(),
        "symbol": args.symbol,
        "deadline": args.deadline,
        "inputs": {
            "buy_eth_wei": buy_eth.to_string(),
            "min_tokens_out_raw": min_tokens_out.to_string(),
            "min_eth_out_wei": min_eth_out.to_string(),
            "sell_amount_raw": sell_amount.to_string()
        },
        "gas_policy": {
            "kind": "eip1559_public_priority_fee",
            "max_fee_per_gas_wei": max_fee_per_gas.to_string(),
            "priority_fee_per_gas_wei": priority_fee_per_gas.to_string(),
            "block_base_fee_per_gas_wei": base_fee_per_gas.map(|value| value.to_string()),
            "direct_coinbase_payment_supported": false,
            "note": "Direct-router baseline uses the same public EIP-1559 fee model. It is not the production trading surface."
        },
        "balances_before": balances_before.to_json(),
        "direct_buy": buy_result.as_ref().map(|result| tx_report(result, base_fee_per_gas)),
        "approve": tx_report(&approve_result, base_fee_per_gas),
        "direct_sell": tx_report(&sell_result, base_fee_per_gas),
        "balances_after": balances_after.to_json(),
        "chain_state": {
            "transactions_executed": state.transaction_count,
            "total_gas_used": state.total_gas_used
        }
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[derive(Debug, Clone)]
struct BalanceSnapshot {
    owner_token: U256,
    owner_eth: U256,
}

impl BalanceSnapshot {
    fn to_json(&self) -> Value {
        json!({
            "owner_token_raw": self.owner_token.to_string(),
            "owner_eth_wei": self.owner_eth.to_string()
        })
    }
}

fn balances(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
) -> Result<BalanceSnapshot> {
    Ok(BalanceSnapshot {
        owner_token: erc20_balance(chain, token, owner)?,
        owner_eth: chain.eth_balance(owner)?,
    })
}

fn tx_report(result: &FullSimulationResult, base_fee_per_gas: Option<u128>) -> Value {
    let effective_gas_price = result.effective_gas_price.unwrap_or_default();
    let effective_priority_fee = base_fee_per_gas
        .map(|base_fee| effective_gas_price.saturating_sub(base_fee))
        .unwrap_or_default();
    let priority_spend = U256::from(result.gas_used) * U256::from(effective_priority_fee);
    json!({
        "success": result.success,
        "gas_used": result.gas_used,
        "logs": result.logs.len(),
        "revert_reason": result.revert_reason,
        "effective_gas_price_wei": result.effective_gas_price.map(|value| value.to_string()),
        "effective_priority_fee_per_gas_wei": effective_priority_fee.to_string(),
        "estimated_priority_fee_spend_wei": priority_spend.to_string()
    })
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

fn erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
) -> Result<U256> {
    let data = Bytes::from(balanceOfCall { owner }.abi_encode());
    let result = chain.simulate_view_call(token, data)?;
    if !result.success {
        return Err(eyre!("balanceOf({owner}) failed for token {token}"));
    }
    decode_u256(&result.output, "balanceOf output")
}

fn apply_eip1559_gas_policy(
    tx: &mut UnsignedTransaction,
    max_fee_per_gas: u128,
    priority_fee_per_gas: u128,
) {
    tx.gas_price = None;
    tx.max_fee_per_gas = Some(max_fee_per_gas);
    tx.max_priority_fee_per_gas = Some(priority_fee_per_gas);
}

fn maybe_print_trace(label: &str, result: &FullSimulationResult, enabled: bool) {
    if enabled && !result.success {
        eprintln!("{label} call trace:\n{:#?}", result.call_trace);
    }
}

fn decode_u256(output: &Bytes, label: &str) -> Result<U256> {
    if output.len() < 32 {
        return Err(eyre!("{label} is shorter than 32 bytes"));
    }
    Ok(U256::from_be_slice(&output[..32]))
}
