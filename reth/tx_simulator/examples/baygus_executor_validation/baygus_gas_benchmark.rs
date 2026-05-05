use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result};
use std::str::FromStr;
use tx_simulator::{
    tx_builders::{
        baygus_executor::{
            build_baygus_execute_tx, build_permit2_approve_tx, BaygusExecutionPlan,
            BaygusV3ExactInputSingle,
        },
        uniswap_v2::{self, Router as V2Router},
        uniswap_v3,
        uniswap_v4::{
            build_baygus_executor_deploy_tx, build_token_approval_tx, build_weth_deposit_tx,
            compute_contract_address,
        },
    },
    types::SimulationResult,
    TxSimulator, UnsignedTransaction,
};

const ONE_ETH_WEI: u128 = 1_000_000_000_000_000_000;
const BENCH_GAS_PRICE_WEI: u128 = 50_000_000_000;

const BUYER: &str = "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5";
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const UNISWAP_V4_POOL_MANAGER: &str = "0x000000000004444C5DC75cB358380d2E3de08a90";
const PERMIT2: &str = "0x000000000022D473030F116dDEE9F6B43aC78BA3";

const ERC20_BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];

#[derive(Debug, Clone, Copy)]
enum RouteKind {
    UniswapV2,
    SushiswapV2,
    UniswapV3 { fee: u32 },
}

#[derive(Debug, Clone, Copy)]
enum PullMode {
    Erc20Allowance,
    Permit2Allowance,
}

#[derive(Debug, Clone, Copy)]
struct BenchRoute {
    label: &'static str,
    kind: RouteKind,
    output_token: Address,
    output_symbol: &'static str,
    output_decimals: usize,
}

#[derive(Debug)]
struct GasBenchmark {
    label: &'static str,
    output_symbol: &'static str,
    output_decimals: usize,
    direct_setup_gas: u64,
    direct_execute_gas: u64,
    direct_output: U256,
    executor_deploy_gas: u64,
    executor_setup_gas: u64,
    executor_execute_gas: u64,
    executor_output: U256,
    permit2_deploy_gas: u64,
    permit2_setup_gas: u64,
    permit2_execute_gas: u64,
    permit2_output: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let reth_datadir = match std::env::var("RETH_DATADIR") {
        Ok(path) => path,
        Err(_) => tx_simulator::config::repo::reth_datadir()?,
    };

    let simulator = TxSimulator::new(&reth_datadir)?;
    let latest_block = simulator.get_latest_block()?;
    let block_number = configured_simulation_block(latest_block)?;

    let buyer = parse_address(BUYER, "buyer")?;
    let weth = parse_address(WETH, "WETH")?;
    let usdc = parse_address(USDC, "USDC")?;
    let usdt = parse_address(USDT, "USDT")?;
    let pool_manager = parse_address(UNISWAP_V4_POOL_MANAGER, "Uniswap V4 PoolManager")?;
    let permit2 = parse_address(PERMIT2, "Permit2")?;
    let amount_in = U256::from(ONE_ETH_WEI);

    let routes = vec![
        BenchRoute {
            label: "Uniswap V2 WETH/USDC",
            kind: RouteKind::UniswapV2,
            output_token: usdc,
            output_symbol: "USDC",
            output_decimals: 6,
        },
        BenchRoute {
            label: "Uniswap V2 WETH/USDT",
            kind: RouteKind::UniswapV2,
            output_token: usdt,
            output_symbol: "USDT",
            output_decimals: 6,
        },
        BenchRoute {
            label: "SushiSwap V2 WETH/USDC",
            kind: RouteKind::SushiswapV2,
            output_token: usdc,
            output_symbol: "USDC",
            output_decimals: 6,
        },
        BenchRoute {
            label: "Uniswap V3 5bp WETH/USDC",
            kind: RouteKind::UniswapV3 { fee: 500 },
            output_token: usdc,
            output_symbol: "USDC",
            output_decimals: 6,
        },
    ];

    println!("Baygus Executor gas benchmark");
    println!("=============================");
    println!("Reth datadir         : {reth_datadir}");
    println!("Simulation block     : {block_number}");
    if block_number != latest_block {
        println!("Latest block         : {latest_block}");
    }
    println!("Buyer                : {buyer}");
    println!(
        "Amount in            : {} WETH",
        format_units(amount_in, 18, 6)
    );
    println!(
        "Benchmark gas price  : {} gwei",
        BENCH_GAS_PRICE_WEI / 1_000_000_000
    );

    let mut results = Vec::new();
    for route in routes {
        let direct = run_direct(&simulator, block_number, buyer, weth, amount_in, route).await?;
        let executor = run_executor(
            &simulator,
            block_number,
            buyer,
            weth,
            permit2,
            pool_manager,
            amount_in,
            route,
            PullMode::Erc20Allowance,
        )
        .await?;
        let permit2_executor = run_executor(
            &simulator,
            block_number,
            buyer,
            weth,
            permit2,
            pool_manager,
            amount_in,
            route,
            PullMode::Permit2Allowance,
        )
        .await?;
        results.push(GasBenchmark {
            label: route.label,
            output_symbol: route.output_symbol,
            output_decimals: route.output_decimals,
            direct_setup_gas: direct.setup_gas,
            direct_execute_gas: direct.execute_gas,
            direct_output: direct.output,
            executor_deploy_gas: executor.deploy_gas,
            executor_setup_gas: executor.setup_gas,
            executor_execute_gas: executor.execute_gas,
            executor_output: executor.output,
            permit2_deploy_gas: permit2_executor.deploy_gas,
            permit2_setup_gas: permit2_executor.setup_gas,
            permit2_execute_gas: permit2_executor.execute_gas,
            permit2_output: permit2_executor.output,
        });
    }

    println!();
    println!("Execution Gas");
    println!("-------------");
    println!(
        "{:<32} {:>12} {:>12} {:>12} {:>12} {:>12}",
        "Route", "direct", "exec erc20", "exec p2", "erc20 ovh", "p2 ovh"
    );
    for result in &results {
        println!(
            "{:<32} {:>12} {:>12} {:>12} {:>+12} {:>+12}",
            result.label,
            result.direct_execute_gas,
            result.executor_execute_gas,
            result.permit2_execute_gas,
            result.executor_execute_gas as i128 - result.direct_execute_gas as i128,
            result.permit2_execute_gas as i128 - result.direct_execute_gas as i128
        );
    }

    println!();
    println!("Execution Cost at Benchmark Gas Price");
    println!("-------------------------------------");
    println!(
        "{:<32} {:>12} {:>12} {:>12}",
        "Route", "direct ETH", "erc20 ETH", "p2 ETH"
    );
    for result in &results {
        println!(
            "{:<32} {:>12} {:>12} {:>12}",
            result.label,
            gas_cost_eth(result.direct_execute_gas),
            gas_cost_eth(result.executor_execute_gas),
            gas_cost_eth(result.permit2_execute_gas)
        );
    }

    println!();
    println!("Setup Gas");
    println!("---------");
    println!(
        "{:<32} {:>12} {:>12} {:>12} {:>12} {:>12}",
        "Route", "direct", "erc20", "p2", "deploy", "p2 deploy"
    );
    for result in &results {
        println!(
            "{:<32} {:>12} {:>12} {:>12} {:>12} {:>12}",
            result.label,
            result.direct_setup_gas,
            result.executor_setup_gas,
            result.permit2_setup_gas,
            result.executor_deploy_gas,
            result.permit2_deploy_gas
        );
    }

    println!();
    println!("Outputs");
    println!("-------");
    for result in &results {
        println!(
            "{:<32} direct {} {} | erc20 {} {} | p2 {} {} | diffs erc20 {} p2 {}",
            result.label,
            format_units(result.direct_output, result.output_decimals, 6),
            result.output_symbol,
            format_units(result.executor_output, result.output_decimals, 6),
            result.output_symbol,
            format_units(result.permit2_output, result.output_decimals, 6),
            result.output_symbol,
            diff_label(result.executor_output, result.direct_output),
            diff_label(result.permit2_output, result.direct_output)
        );
    }

    Ok(())
}

#[derive(Debug)]
struct DirectBenchmark {
    setup_gas: u64,
    execute_gas: u64,
    output: U256,
}

#[derive(Debug)]
struct ExecutorBenchmark {
    deploy_gas: u64,
    setup_gas: u64,
    execute_gas: u64,
    output: U256,
}

async fn run_direct(
    simulator: &TxSimulator,
    block_number: u64,
    buyer: Address,
    weth: Address,
    amount_in: U256,
    route: BenchRoute,
) -> Result<DirectBenchmark> {
    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;

    let mut deposit_tx = build_weth_deposit_tx(buyer, weth, amount_in);
    apply_simple_gas_policy(&mut deposit_tx);
    let deposit = chain.step(deposit_tx).await?;
    ensure_success(&format!("{} direct WETH deposit", route.label), &deposit)?;

    let mut approve_tx = build_direct_approval(route, buyer, weth);
    apply_simple_gas_policy(&mut approve_tx);
    let approve = chain.step(approve_tx).await?;
    ensure_success(&format!("{} direct approval", route.label), &approve)?;

    let before = erc20_balance_of(&mut chain, route.output_token, buyer)?;
    let mut execute_tx = build_direct_swap(route, buyer, weth, amount_in);
    apply_simple_gas_policy(&mut execute_tx);
    let execute = chain.step(execute_tx).await?;
    ensure_success(&format!("{} direct execute", route.label), &execute)?;
    let after = erc20_balance_of(&mut chain, route.output_token, buyer)?;

    Ok(DirectBenchmark {
        setup_gas: deposit.gas_used + approve.gas_used,
        execute_gas: execute.gas_used,
        output: after
            .checked_sub(before)
            .ok_or_else(|| eyre!("{} direct output balance decreased", route.label))?,
    })
}

async fn run_executor(
    simulator: &TxSimulator,
    block_number: u64,
    buyer: Address,
    weth: Address,
    permit2: Address,
    pool_manager: Address,
    amount_in: U256,
    route: BenchRoute,
    pull_mode: PullMode,
) -> Result<ExecutorBenchmark> {
    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;
    let executor_address = compute_contract_address(buyer, chain.account_nonce(buyer)?);

    let mut deploy_tx = build_baygus_executor_deploy_tx(buyer, pool_manager)?;
    apply_simple_gas_policy(&mut deploy_tx);
    let deploy = chain.step(deploy_tx).await?;
    ensure_success("Baygus Executor deploy", &deploy)?;

    if !chain.account_has_code(executor_address)? {
        return Err(eyre!(
            "executor deploy succeeded but no bytecode is visible at {executor_address}"
        ));
    }

    let approval_gas = match pull_mode {
        PullMode::Erc20Allowance => {
            let mut approve_tx = build_token_approval_tx(buyer, weth, executor_address, U256::MAX);
            apply_simple_gas_policy(&mut approve_tx);
            let approve = chain.step(approve_tx).await?;
            ensure_success("WETH approval for executor", &approve)?;
            approve.gas_used
        }
        PullMode::Permit2Allowance => {
            let mut token_approve_tx = build_token_approval_tx(buyer, weth, permit2, U256::MAX);
            apply_simple_gas_policy(&mut token_approve_tx);
            let token_approve = chain.step(token_approve_tx).await?;
            ensure_success("WETH approval for Permit2", &token_approve)?;

            let mut permit2_approve_tx = build_permit2_approve_tx(
                buyer,
                permit2,
                weth,
                executor_address,
                max_uint160(),
                max_uint48(),
            );
            apply_simple_gas_policy(&mut permit2_approve_tx);
            let permit2_approve = chain.step(permit2_approve_tx).await?;
            ensure_success("Permit2 allowance for executor", &permit2_approve)?;

            token_approve.gas_used + permit2_approve.gas_used
        }
    };

    let mut deposit_tx = build_weth_deposit_tx(buyer, weth, amount_in);
    apply_simple_gas_policy(&mut deposit_tx);
    let deposit = chain.step(deposit_tx).await?;
    ensure_success(&format!("{} executor WETH deposit", route.label), &deposit)?;

    let before = erc20_balance_of(&mut chain, route.output_token, buyer)?;
    let mut execute_tx =
        build_executor_swap(route, buyer, executor_address, weth, amount_in, pull_mode);
    apply_simple_gas_policy(&mut execute_tx);
    let execute = chain.step(execute_tx).await?;
    ensure_success(&format!("{} executor execute", route.label), &execute)?;
    let after = erc20_balance_of(&mut chain, route.output_token, buyer)?;

    Ok(ExecutorBenchmark {
        deploy_gas: deploy.gas_used,
        setup_gas: approval_gas + deposit.gas_used,
        execute_gas: execute.gas_used,
        output: after
            .checked_sub(before)
            .ok_or_else(|| eyre!("{} executor output balance decreased", route.label))?,
    })
}

fn build_direct_approval(route: BenchRoute, buyer: Address, weth: Address) -> UnsignedTransaction {
    match route.kind {
        RouteKind::UniswapV2 => {
            uniswap_v2::build_approve_v2(V2Router::UniswapV2, buyer, weth, U256::MAX)
        }
        RouteKind::SushiswapV2 => {
            uniswap_v2::build_approve_v2(V2Router::SushiswapV2, buyer, weth, U256::MAX)
        }
        RouteKind::UniswapV3 { .. } => uniswap_v3::build_approve_v3(buyer, weth, U256::MAX),
    }
}

fn build_direct_swap(
    route: BenchRoute,
    buyer: Address,
    weth: Address,
    amount_in: U256,
) -> UnsignedTransaction {
    match route.kind {
        RouteKind::UniswapV2 => uniswap_v2::build_token_to_token_swap_v2_with_min_out(
            V2Router::UniswapV2,
            buyer,
            weth,
            route.output_token,
            amount_in,
            U256::ZERO,
            u64::MAX,
        ),
        RouteKind::SushiswapV2 => uniswap_v2::build_token_to_token_swap_v2_with_min_out(
            V2Router::SushiswapV2,
            buyer,
            weth,
            route.output_token,
            amount_in,
            U256::ZERO,
            u64::MAX,
        ),
        RouteKind::UniswapV3 { fee } => uniswap_v3::build_token_to_token_swap_v3_with_min_out(
            buyer,
            weth,
            route.output_token,
            amount_in,
            fee,
            U256::ZERO,
            u64::MAX,
        ),
    }
}

fn build_executor_swap(
    route: BenchRoute,
    buyer: Address,
    executor_address: Address,
    weth: Address,
    amount_in: U256,
    pull_mode: PullMode,
) -> UnsignedTransaction {
    let mut plan = BaygusExecutionPlan::new();
    match pull_mode {
        PullMode::Erc20Allowance => {
            plan.transfer_from(weth, amount_in);
        }
        PullMode::Permit2Allowance => {
            plan.permit2_transfer_from(weth, amount_in);
        }
    }
    match route.kind {
        RouteKind::UniswapV2 => {
            plan.v2_swap(
                amount_in,
                U256::ZERO,
                [weth, route.output_token],
                executor_address,
            );
        }
        RouteKind::SushiswapV2 => {
            plan.sushiswap_swap(
                amount_in,
                U256::ZERO,
                [weth, route.output_token],
                executor_address,
            );
        }
        RouteKind::UniswapV3 { fee } => {
            plan.v3_swap(BaygusV3ExactInputSingle {
                token_in: weth,
                token_out: route.output_token,
                fee,
                recipient: executor_address,
                deadline: U256::MAX,
                amount_in,
                amount_out_minimum: U256::ZERO,
                sqrt_price_limit_x96: U256::ZERO,
            });
        }
    }
    plan.sweep(route.output_token, buyer, U256::ZERO);
    build_baygus_execute_tx(executor_address, buyer, &plan)
}

fn configured_simulation_block(latest_block: u64) -> Result<u64> {
    match std::env::var("BAYGUS_SIM_BLOCK") {
        Ok(value) => {
            let requested = value.trim().parse::<u64>().map_err(|err| {
                eyre!("invalid BAYGUS_SIM_BLOCK value {value:?}; expected block number: {err}")
            })?;
            if requested > latest_block {
                return Err(eyre!(
                    "BAYGUS_SIM_BLOCK {requested} is ahead of latest local block {latest_block}"
                ));
            }
            Ok(requested)
        }
        Err(std::env::VarError::NotPresent) => Ok(latest_block),
        Err(err) => Err(eyre!("failed to read BAYGUS_SIM_BLOCK: {err}")),
    }
}

fn ensure_success(label: &str, result: &SimulationResult) -> Result<()> {
    if result.success {
        Ok(())
    } else {
        Err(eyre!(
            "{label} failed: {}",
            result
                .revert_reason
                .as_deref()
                .unwrap_or("transaction reverted without decoded reason")
        ))
    }
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

fn parse_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value).map_err(|err| eyre!("invalid {label} address {value}: {err}"))
}

fn apply_simple_gas_policy(tx: &mut UnsignedTransaction) {
    if tx.gas.is_none() {
        tx.gas = Some(5_000_000);
    }
    if tx.max_fee_per_gas.is_none() && tx.gas_price.is_none() {
        tx.gas_price = Some(BENCH_GAS_PRICE_WEI);
    }
}

fn max_uint160() -> U256 {
    (U256::from(1u8) << 160) - U256::from(1u8)
}

fn max_uint48() -> u64 {
    (1u64 << 48) - 1
}

fn gas_cost_eth(gas: u64) -> String {
    format_units(U256::from(gas) * U256::from(BENCH_GAS_PRICE_WEI), 18, 6)
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

fn diff_label(actual: U256, expected: U256) -> String {
    if actual >= expected {
        format!("+{}", actual - expected)
    } else {
        format!("-{}", expected - actual)
    }
}
