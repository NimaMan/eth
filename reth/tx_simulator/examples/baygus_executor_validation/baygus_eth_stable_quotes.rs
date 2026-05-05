use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result};
use std::str::FromStr;
use tx_simulator::{
    tx_builders::{
        baygus_executor::{build_baygus_execute_tx, BaygusExecutionPlan, BaygusV3ExactInputSingle},
        uniswap_v4::{
            build_baygus_executor_deploy_tx, build_baygus_executor_multihop_tx,
            build_baygus_executor_single_hop_exact_input_call, build_token_approval_tx,
            build_weth_deposit_tx, compute_contract_address, UniswapV4BaygusSingleHopRequest,
            UniswapV4PoolKey,
        },
    },
    types::SimulationResult,
    TxSimulator, UnsignedTransaction,
};

const ONE_ETH_WEI: u128 = 1_000_000_000_000_000_000;

const BUYER: &str = "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5";
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
const UNISWAP_V3_QUOTER: &str = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6";
const UNISWAP_V4_POOL_MANAGER: &str = "0x000000000004444C5DC75cB358380d2E3de08a90";

const ERC20_BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
const UNISWAP_V2_GET_AMOUNTS_OUT_SELECTOR: [u8; 4] = [0xd0, 0x6c, 0xa6, 0x1f];
const UNISWAP_V3_QUOTE_EXACT_INPUT_SINGLE_SELECTOR: [u8; 4] = [0xf7, 0x72, 0x9d, 0x43];

#[derive(Clone)]
enum QuoteKind {
    V2Router { router: Address },
    V3Quoter { quoter: Address, fee: u32 },
    SimulationOnly,
}

#[derive(Clone)]
enum ExecutionKind {
    UniswapV2,
    SushiswapV2,
    UniswapV3 { fee: u32 },
    UniswapV4 { pool_key: UniswapV4PoolKey },
}

#[derive(Clone)]
struct Route {
    label: &'static str,
    output_symbol: &'static str,
    output_decimals: usize,
    output_token: Address,
    quote: QuoteKind,
    execution: ExecutionKind,
    block_hint: Option<u64>,
    required: bool,
}

struct RouteReport {
    label: &'static str,
    output_symbol: &'static str,
    output_decimals: usize,
    quote: Option<U256>,
    output: U256,
    deposit_gas: u64,
    execute_gas: u64,
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
    let v2_router = parse_address(UNISWAP_V2_ROUTER, "Uniswap V2 router")?;
    let sushi_router = parse_address(SUSHISWAP_ROUTER, "SushiSwap router")?;
    let v3_quoter = parse_address(UNISWAP_V3_QUOTER, "Uniswap V3 quoter")?;
    let pool_manager = parse_address(UNISWAP_V4_POOL_MANAGER, "Uniswap V4 pool manager")?;
    let amount_in = U256::from(ONE_ETH_WEI);

    let routes = eth_stable_routes(weth, usdc, usdt, v2_router, sushi_router, v3_quoter);

    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;
    let deployer_nonce = chain.account_nonce(buyer)?;
    let executor_address = compute_contract_address(buyer, deployer_nonce);

    println!("Baygus ETH -> stable validation");
    println!("===============================");
    println!("Reth datadir      : {reth_datadir}");
    println!("Simulation block  : {block_number}");
    println!("Buyer/deployer    : {buyer}");
    if block_number != latest_block {
        println!("Latest block      : {latest_block}");
    }
    println!("Baygus executor   : {executor_address}");
    println!(
        "Amount in         : {} WETH per route",
        format_units(amount_in, 18, 6)
    );
    println!("Routes            : {}", routes.len());

    let mut deploy_tx = build_baygus_executor_deploy_tx(buyer, pool_manager)?;
    apply_simple_gas_policy(&mut deploy_tx);
    let deploy_result = chain.step(deploy_tx).await?;
    ensure_success("Baygus deploy", &deploy_result)?;

    if !chain.account_has_code(executor_address)? {
        return Err(eyre!(
            "Baygus deploy reported success but no code is visible at {executor_address}"
        ));
    }

    let mut approve_tx = build_token_approval_tx(buyer, weth, executor_address, U256::MAX);
    apply_simple_gas_policy(&mut approve_tx);
    let approve_result = chain.step(approve_tx).await?;
    ensure_success("WETH approval", &approve_result)?;

    println!();
    println!("Setup");
    println!("-----");
    println!("Deploy gas        : {}", deploy_result.gas_used);
    println!("WETH approve gas  : {}", approve_result.gas_used);

    let mut reports = Vec::new();
    let mut failures = Vec::new();
    let mut findings = Vec::new();
    for route in routes {
        match run_route(
            &mut chain,
            &route,
            buyer,
            executor_address,
            weth,
            amount_in,
            block_number,
        )
        .await
        {
            Ok(report) => reports.push(report),
            Err(err) => {
                if route.required {
                    failures.push((route.label, err.to_string()));
                } else {
                    findings.push((route.label, err.to_string()));
                }
            }
        }
    }

    println!();
    println!("Prices");
    println!("------");
    for report in &reports {
        let output = format_units(report.output, report.output_decimals, 6);
        match report.quote {
            Some(quote) => {
                let quote_fmt = format_units(quote, report.output_decimals, 6);
                println!(
                    "{:<32} {} {} | quote {} | diff {} | gas deposit {} execute {}",
                    report.label,
                    output,
                    report.output_symbol,
                    quote_fmt,
                    diff_label(report.output, quote),
                    report.deposit_gas,
                    report.execute_gas
                );
            }
            None => {
                println!(
                    "{:<32} {} {} | quote simulation-only | gas deposit {} execute {}",
                    report.label,
                    output,
                    report.output_symbol,
                    report.deposit_gas,
                    report.execute_gas
                );
            }
        }
    }

    if !failures.is_empty() {
        println!();
        println!("Failures");
        println!("--------");
        for (label, reason) in &failures {
            println!("{label}: {reason}");
        }
        return Err(eyre!("{} route(s) failed", failures.len()));
    }

    if !findings.is_empty() {
        println!();
        println!("Investigations");
        println!("--------------");
        for (label, reason) in &findings {
            println!("{label}: {reason}");
        }
    }

    for report in &reports {
        if let Some(quote) = report.quote {
            if report.output != quote {
                return Err(eyre!(
                    "{} output does not match quote: got {}, expected {}",
                    report.label,
                    report.output,
                    quote
                ));
            }
        }
    }

    Ok(())
}

fn eth_stable_routes(
    weth: Address,
    usdc: Address,
    usdt: Address,
    v2_router: Address,
    sushi_router: Address,
    v3_quoter: Address,
) -> Vec<Route> {
    vec![
        Route {
            label: "Uniswap V2 WETH/USDC",
            output_symbol: "USDC",
            output_decimals: 6,
            output_token: usdc,
            quote: QuoteKind::V2Router { router: v2_router },
            execution: ExecutionKind::UniswapV2,
            block_hint: None,
            required: true,
        },
        Route {
            label: "Uniswap V2 WETH/USDT",
            output_symbol: "USDT",
            output_decimals: 6,
            output_token: usdt,
            quote: QuoteKind::V2Router { router: v2_router },
            execution: ExecutionKind::UniswapV2,
            block_hint: None,
            required: true,
        },
        Route {
            label: "SushiSwap V2 WETH/USDC",
            output_symbol: "USDC",
            output_decimals: 6,
            output_token: usdc,
            quote: QuoteKind::V2Router {
                router: sushi_router,
            },
            execution: ExecutionKind::SushiswapV2,
            block_hint: None,
            required: true,
        },
        Route {
            label: "Uniswap V3 5bp WETH/USDC",
            output_symbol: "USDC",
            output_decimals: 6,
            output_token: usdc,
            quote: QuoteKind::V3Quoter {
                quoter: v3_quoter,
                fee: 500,
            },
            execution: ExecutionKind::UniswapV3 { fee: 500 },
            block_hint: None,
            required: true,
        },
        Route {
            label: "Uniswap V4 4.9bp WETH/USDC",
            output_symbol: "USDC",
            output_decimals: 6,
            output_token: usdc,
            quote: QuoteKind::SimulationOnly,
            execution: ExecutionKind::UniswapV4 {
                pool_key: UniswapV4PoolKey {
                    currency0: usdc,
                    currency1: weth,
                    fee: 490,
                    tick_spacing: 10,
                    hooks: Address::ZERO,
                },
            },
            block_hint: Some(23_560_197),
            required: false,
        },
        Route {
            label: "Uniswap V4 4.5bp WETH/USDC",
            output_symbol: "USDC",
            output_decimals: 6,
            output_token: usdc,
            quote: QuoteKind::SimulationOnly,
            execution: ExecutionKind::UniswapV4 {
                pool_key: UniswapV4PoolKey {
                    currency0: usdc,
                    currency1: weth,
                    fee: 450,
                    tick_spacing: 9,
                    hooks: Address::ZERO,
                },
            },
            block_hint: Some(23_566_463),
            required: false,
        },
    ]
}

async fn run_route(
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    route: &Route,
    buyer: Address,
    executor_address: Address,
    weth: Address,
    amount_in: U256,
    block_number: u64,
) -> Result<RouteReport> {
    let quote = quote_route(chain, route, weth, amount_in)?;

    let mut deposit_tx = build_weth_deposit_tx(buyer, weth, amount_in);
    apply_simple_gas_policy(&mut deposit_tx);
    let deposit_result = chain.step(deposit_tx).await?;
    ensure_success(&format!("{} WETH deposit", route.label), &deposit_result)?;

    let before = erc20_balance_of(chain, route.output_token, buyer)?;
    let mut execute_tx = build_execute_tx(route, buyer, executor_address, weth, amount_in)?;
    apply_simple_gas_policy(&mut execute_tx);
    let execute_result = chain.step_with_trace(execute_tx).await?;
    if !execute_result.success {
        let reason = execute_result
            .revert_reason
            .as_deref()
            .unwrap_or("transaction reverted without decoded reason");
        if trace_failures_enabled() {
            return Err(eyre!(
                "{} execute failed: {}\n{:#?}",
                route.label,
                reason,
                execute_result.call_trace
            ));
        }
        return Err(eyre!("{} execute failed: {}", route.label, reason));
    }
    if trace_successes_enabled() {
        println!();
        println!("Trace for {}", route.label);
        println!("{:#?}", execute_result.call_trace);
    }
    let after = erc20_balance_of(chain, route.output_token, buyer)?;

    let output = after
        .checked_sub(before)
        .ok_or_else(|| eyre!("{} output balance decreased", route.label))?;
    if output.is_zero() {
        return Err(eyre!(
            "{}",
            zero_output_diagnostic(route, block_number, execute_result.gas_used)
        ));
    }

    Ok(RouteReport {
        label: route.label,
        output_symbol: route.output_symbol,
        output_decimals: route.output_decimals,
        quote,
        output,
        deposit_gas: deposit_result.gas_used,
        execute_gas: execute_result.gas_used,
    })
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

fn zero_output_diagnostic(route: &Route, block_number: u64, execute_gas: u64) -> String {
    let mut diagnostic = format!(
        "{} executed successfully at block {} but produced zero {} (execute gas {})",
        route.label, block_number, route.output_symbol, execute_gas
    );

    if let ExecutionKind::UniswapV4 { pool_key } = &route.execution {
        diagnostic.push_str(&format!(
            "; v4 pool key currency0={} currency1={} fee={} tickSpacing={} hooks={}",
            pool_key.currency0,
            pool_key.currency1,
            pool_key.fee,
            pool_key.tick_spacing,
            pool_key.hooks
        ));
    }

    if let Some(block_hint) = route.block_hint {
        diagnostic.push_str(&format!(
            "; historical hint: rerun with BAYGUS_SIM_BLOCK={block_hint}"
        ));
    }

    diagnostic
}

fn trace_failures_enabled() -> bool {
    std::env::var("BAYGUS_TRACE_FAILURE")
        .map(|value| {
            let lowered = value.trim().to_ascii_lowercase();
            lowered == "1" || lowered == "true" || lowered == "yes"
        })
        .unwrap_or(false)
}

fn trace_successes_enabled() -> bool {
    std::env::var("BAYGUS_TRACE_SUCCESS")
        .map(|value| {
            let lowered = value.trim().to_ascii_lowercase();
            lowered == "1" || lowered == "true" || lowered == "yes"
        })
        .unwrap_or(false)
}

fn quote_route(
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    route: &Route,
    weth: Address,
    amount_in: U256,
) -> Result<Option<U256>> {
    match route.quote {
        QuoteKind::V2Router { router } => {
            uniswap_v2_get_amounts_out(chain, router, amount_in, [weth, route.output_token])
                .map(Some)
        }
        QuoteKind::V3Quoter { quoter, fee } => uniswap_v3_quote_exact_input_single(
            chain,
            quoter,
            weth,
            route.output_token,
            fee,
            amount_in,
        )
        .map(Some),
        QuoteKind::SimulationOnly => Ok(None),
    }
}

fn build_execute_tx(
    route: &Route,
    buyer: Address,
    executor_address: Address,
    weth: Address,
    amount_in: U256,
) -> Result<UnsignedTransaction> {
    match &route.execution {
        ExecutionKind::UniswapV2 => {
            let mut plan = BaygusExecutionPlan::new();
            plan.transfer_from(weth, amount_in)
                .v2_swap(
                    amount_in,
                    U256::ZERO,
                    [weth, route.output_token],
                    executor_address,
                )
                .sweep(route.output_token, buyer, U256::ZERO);
            Ok(build_baygus_execute_tx(executor_address, buyer, &plan))
        }
        ExecutionKind::SushiswapV2 => {
            let mut plan = BaygusExecutionPlan::new();
            plan.transfer_from(weth, amount_in)
                .sushiswap_swap(
                    amount_in,
                    U256::ZERO,
                    [weth, route.output_token],
                    executor_address,
                )
                .sweep(route.output_token, buyer, U256::ZERO);
            Ok(build_baygus_execute_tx(executor_address, buyer, &plan))
        }
        ExecutionKind::UniswapV3 { fee } => {
            let mut plan = BaygusExecutionPlan::new();
            plan.transfer_from(weth, amount_in)
                .v3_swap(BaygusV3ExactInputSingle {
                    token_in: weth,
                    token_out: route.output_token,
                    fee: *fee,
                    recipient: executor_address,
                    deadline: U256::MAX,
                    amount_in,
                    amount_out_minimum: U256::ZERO,
                    sqrt_price_limit_x96: U256::ZERO,
                })
                .sweep(route.output_token, buyer, U256::ZERO);
            Ok(build_baygus_execute_tx(executor_address, buyer, &plan))
        }
        ExecutionKind::UniswapV4 { pool_key } => {
            let request = UniswapV4BaygusSingleHopRequest {
                pool_key: pool_key.clone(),
                token_in: weth,
                token_out: route.output_token,
                amount_in,
                recipient: buyer,
                min_output: None,
                hook_adapter: Address::ZERO,
                hook_data: Vec::new(),
                sqrt_price_limit_x96: None,
            };
            let call = build_baygus_executor_single_hop_exact_input_call(&request)?;
            build_baygus_executor_multihop_tx(executor_address, buyer, &call.params, call.eth_value)
        }
    }
}

fn parse_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value).map_err(|err| eyre!("invalid {label} address {value}: {err}"))
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

fn uniswap_v2_get_amounts_out(
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    router: Address,
    amount_in: U256,
    path: [Address; 2],
) -> Result<U256> {
    let mut data = Vec::with_capacity(4 + 32 * 5);
    data.extend_from_slice(&UNISWAP_V2_GET_AMOUNTS_OUT_SELECTOR);
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(64).to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(path.len()).to_be_bytes::<32>());
    for token in path {
        data.extend_from_slice(&pad_address(token));
    }

    let result = chain.simulate_view_call(router, Bytes::from(data))?;
    if !result.success {
        return Err(eyre!("Uniswap V2 getAmountsOut view call failed"));
    }

    let output = result.output.as_ref();
    let len = decode_u256_word(output, 32, "getAmountsOut length")?;
    if len != U256::from(2) {
        return Err(eyre!("expected two getAmountsOut values, got {len}"));
    }
    decode_u256_word(output, 96, "getAmountsOut amountOut")
}

fn uniswap_v3_quote_exact_input_single(
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    quoter: Address,
    token_in: Address,
    token_out: Address,
    fee: u32,
    amount_in: U256,
) -> Result<U256> {
    let mut data = Vec::with_capacity(4 + 32 * 5);
    data.extend_from_slice(&UNISWAP_V3_QUOTE_EXACT_INPUT_SINGLE_SELECTOR);
    data.extend_from_slice(&pad_address(token_in));
    data.extend_from_slice(&pad_address(token_out));
    data.extend_from_slice(&U256::from(fee).to_be_bytes::<32>());
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());

    let result = chain.simulate_view_call(quoter, Bytes::from(data))?;
    if !result.success {
        return Err(eyre!("Uniswap V3 quoteExactInputSingle call failed"));
    }
    decode_u256_word(result.output.as_ref(), 0, "quoteExactInputSingle output")
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
        tx.gas = Some(5_000_000);
    }
    if tx.max_fee_per_gas.is_none() && tx.gas_price.is_none() {
        tx.gas_price = Some(50_000_000_000);
    }
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
        format!("+{} raw units", actual - expected)
    } else {
        format!("-{} raw units", expected - actual)
    }
}
