use std::sync::Arc;
use std::time::Duration;

use alloy_primitives::{address, Address, Bytes, B256, U256};
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress},
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
};
use eth_live_feed::LiveTokenRuntime;
use eyre::{eyre, Result, WrapErr};
use tokio::time;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::{
    check_can_buy_sell_pool_with_chain, simulate_buy_swap_with_params_and_chain,
    simulate_sell_swap_with_params_and_chain, BuySwapResult, PoolBuySellParameters, PoolType,
    SellSwapResult, UniswapV4PoolConfig as TxUniswapV4PoolConfig,
};
use tx_simulator::{LiveStateStatus, UnsignedTxChainSimulation};

use super::wire::{
    LiveOrderSimulationRequest, LiveOrderSimulationResponse, LivePoolBuySellSimulationConfig,
    LivePoolBuySellSimulationRequest, LivePoolBuySellSimulationResponse,
    LiveUnsignedTxSequenceSimulationRequest, LiveUnsignedTxSequenceSimulationResponse,
    LiveUnsignedTxSimulationRequest, LiveUnsignedTxSimulationResponse,
};

const LIVE_UNSIGNED_TX_SIMULATION_TIMEOUT: Duration = Duration::from_millis(2_500);
const LIVE_UNSIGNED_TX_SEQUENCE_SIMULATION_TIMEOUT: Duration = Duration::from_millis(5_000);
const LIVE_POOL_BUY_SELL_SIMULATION_TIMEOUT: Duration = Duration::from_millis(5_000);
const LIVE_ORDER_SIMULATION_TIMEOUT: Duration = Duration::from_millis(5_000);
const ERC20_DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const USDT_ADDRESS: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
const DAI_ADDRESS: Address = address!("6B175474E89094C44Da98b954EedeAC495271d0F");

pub async fn simulate_live_unsigned_transaction(
    live_tracker: &LiveTokenRuntime,
    request: LiveUnsignedTxSimulationRequest,
) -> Result<LiveUnsignedTxSimulationResponse> {
    if request.block == 0 {
        return Err(eyre!(
            "live unsigned tx simulation requires a non-zero block"
        ));
    }
    let (status, mut chain) = live_tracker
        .live_simulation_chain_at(request.block)
        .await
        .wrap_err_with(|| {
            format!(
                "exact live simulation state is unavailable for block {}",
                request.block
            )
        })?;
    let base_fee = chain.block_base_fee();
    let mut transaction = request.transaction;
    if transaction.gas_price.is_none() && transaction.max_fee_per_gas.is_none() {
        transaction.max_fee_per_gas = base_fee.or(Some(1));
        transaction.max_priority_fee_per_gas.get_or_insert(0);
    }

    let result = time::timeout(
        LIVE_UNSIGNED_TX_SIMULATION_TIMEOUT,
        chain.step_with_trace(transaction),
    )
    .await
    .map_err(|_| {
        eyre!(
            "live unsigned tx simulation timed out after {} ms at block {}",
            LIVE_UNSIGNED_TX_SIMULATION_TIMEOUT.as_millis(),
            status.selected_block_number
        )
    })?
    .wrap_err_with(|| {
        format!(
            "live unsigned tx simulation failed at block {}",
            status.selected_block_number
        )
    })?;

    Ok(LiveUnsignedTxSimulationResponse {
        schema: "eth_live_unsigned_tx_simulation_v1",
        block: status.selected_block_number,
        block_hash: status.selected_block_hash,
        state_source: "chain_server_live_tx_simulator",
        success: result.success,
        gas_used: result.gas_used,
        effective_gas_price_wei: result.effective_gas_price.map(|value| value.to_string()),
        tx_type: result.tx_type,
        revert_reason: result.revert_reason,
        log_count: result.logs.len(),
        logs: result.logs,
        base_fee_per_gas_wei: base_fee.map(|value| value.to_string()),
    })
}

pub async fn simulate_live_unsigned_transaction_sequence(
    live_tracker: &LiveTokenRuntime,
    request: LiveUnsignedTxSequenceSimulationRequest,
) -> Result<LiveUnsignedTxSequenceSimulationResponse> {
    if request.block == 0 {
        return Err(eyre!(
            "live unsigned tx sequence simulation requires a non-zero block"
        ));
    }
    if request.transactions.is_empty() {
        return Err(eyre!(
            "live unsigned tx sequence simulation requires at least one transaction"
        ));
    }

    let (status, chain) = match live_tracker.live_simulation_chain_at(request.block).await {
        Ok(result) => result,
        Err(error) => {
            return Ok(LiveUnsignedTxSequenceSimulationResponse {
                schema: "eth_live_unsigned_tx_sequence_simulation_v1",
                state_available: false,
                unavailable_reason: Some(format!(
                    "exact live tx sequence simulation state is unavailable for block {}: {error}",
                    request.block
                )),
                block: request.block,
                block_hash: None,
                state_source: "chain_server_live_tx_simulator",
                processed_transactions: Vec::new(),
            });
        }
    };
    let tx_processor = Arc::new(TxProcessor::new());
    let block = status.selected_block_number;
    let block_hash = status.selected_block_hash;
    let processed_transactions = time::timeout(
        LIVE_UNSIGNED_TX_SEQUENCE_SIMULATION_TIMEOUT,
        process_unsigned_sequence_with_chain(
            tx_processor,
            request.transactions,
            block,
            request.stop_on_revert,
            chain,
        ),
    )
    .await
    .map_err(|_| {
        eyre!(
            "live unsigned tx sequence simulation timed out after {} ms at block {}",
            LIVE_UNSIGNED_TX_SEQUENCE_SIMULATION_TIMEOUT.as_millis(),
            block
        )
    })??;
    Ok(LiveUnsignedTxSequenceSimulationResponse {
        schema: "eth_live_unsigned_tx_sequence_simulation_v1",
        state_available: true,
        unavailable_reason: None,
        block,
        block_hash,
        state_source: "chain_server_live_tx_simulator",
        processed_transactions,
    })
}

pub async fn simulate_live_pool_buy_sell(
    live_tracker: &LiveTokenRuntime,
    request: LivePoolBuySellSimulationRequest,
) -> Result<LivePoolBuySellSimulationResponse> {
    if request.block == 0 {
        return Err(eyre!(
            "live pool buy/sell simulation requires a non-zero block"
        ));
    }

    let (status, chain) = match live_tracker.live_simulation_chain_at(request.block).await {
        Ok(result) => result,
        Err(error) => {
            return Ok(LivePoolBuySellSimulationResponse {
                schema: "eth_live_pool_buy_sell_simulation_v1",
                state_available: false,
                unavailable_reason: Some(format!(
                    "exact live pool buy/sell state is unavailable for block {}: {error}",
                    request.block
                )),
                block: request.block,
                block_hash: None,
                state_source: "chain_server_live_tx_simulator",
                result: None,
            });
        }
    };
    let simulator = live_tracker.live_tx_simulator().simulator();
    let tx_processor = Arc::new(TxProcessor::new());
    let block = status.selected_block_number;
    let block_hash = status.selected_block_hash;
    let config = pool_buy_sell_config_from_wire(request.config, block);
    let result = time::timeout(
        LIVE_POOL_BUY_SELL_SIMULATION_TIMEOUT,
        check_can_buy_sell_pool_with_chain(simulator, tx_processor, config, chain),
    )
    .await
    .map_err(|_| {
        eyre!(
            "live pool buy/sell simulation timed out after {} ms at block {}",
            LIVE_POOL_BUY_SELL_SIMULATION_TIMEOUT.as_millis(),
            block
        )
    })??;

    Ok(LivePoolBuySellSimulationResponse {
        schema: "eth_live_pool_buy_sell_simulation_v1",
        state_available: true,
        unavailable_reason: None,
        block,
        block_hash,
        state_source: "chain_server_live_tx_simulator",
        result: Some(result),
    })
}

pub async fn simulate_live_order(
    live_tracker: &LiveTokenRuntime,
    request: LiveOrderSimulationRequest,
) -> Result<LiveOrderSimulationResponse> {
    if request.submitted_block == 0 || request.execution_block == 0 {
        return Err(eyre!(
            "live order simulation requires non-zero submitted and execution blocks"
        ));
    }
    if request.execution_block < request.submitted_block {
        return Err(eyre!(
            "live order simulation execution block {} is before submitted block {}",
            request.execution_block,
            request.submitted_block
        ));
    }

    let parent_status = match request.expected_parent_hash {
        Some(expected_parent_hash) => {
            let parent_block = request.execution_block.checked_sub(1).ok_or_else(|| {
                eyre!("live order simulation cannot verify parent hash for genesis execution block")
            })?;
            let Ok((status, _)) = live_tracker.live_simulation_chain_at(parent_block).await else {
                return Ok(unavailable_order_response(
                    request.execution_block,
                    format!(
                        "expected parent state block {parent_block} unavailable while verifying submitted hash {expected_parent_hash}"
                    ),
                ));
            };
            if status.selected_block_hash != Some(expected_parent_hash) {
                let selected_hash = status.selected_block_hash;
                return Ok(unavailable_order_response_with_parent(
                    request.execution_block,
                    Some(status),
                    format!(
                        "reorg: execution parent hash mismatch for block {}: selected={:?} expected={expected_parent_hash}",
                        request.execution_block,
                        selected_hash
                    ),
                ));
            }
            Some(status)
        }
        None => None,
    };

    let (status, chain) = match live_tracker
        .live_simulation_chain_at(request.execution_block)
        .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(unavailable_order_response(
                request.execution_block,
                format!(
                    "exact live order simulation state is unavailable for block {}: {error}",
                    request.execution_block
                ),
            ));
        }
    };
    if status.selected_block_number != request.execution_block {
        let selected_block = status.selected_block_number;
        return Ok(unavailable_order_response_with_status(
            status,
            format!(
                "live simulator selected block {} while request required {}",
                selected_block, request.execution_block
            ),
        ));
    }
    if let Some(expected_block_hash) = request.expected_block_hash {
        if status.selected_block_hash != Some(expected_block_hash) {
            let selected_hash = status.selected_block_hash;
            return Ok(unavailable_order_response_with_status(
                status,
                format!(
                    "reorg: execution block hash mismatch for block {}: selected={:?} expected={expected_block_hash}",
                    request.execution_block,
                    selected_hash
                ),
            ));
        }
    }

    let simulator = live_tracker.live_tx_simulator().simulator();
    let tx_processor = Arc::new(TxProcessor::new());
    let submitted_block = request.submitted_block;
    let execution_block = request.execution_block;
    let report = time::timeout(
        LIVE_ORDER_SIMULATION_TIMEOUT,
        simulate_order_with_chain(
            simulator,
            tx_processor,
            request.order_id,
            request.intent,
            request.pool,
            execution_block,
            request.skip_uneconomic_sell,
            chain,
        ),
    )
    .await
    .map_err(|_| {
        eyre!(
            "live order simulation timed out after {} ms at block {}",
            LIVE_ORDER_SIMULATION_TIMEOUT.as_millis(),
            execution_block
        )
    })??;

    Ok(LiveOrderSimulationResponse {
        schema: "eth_live_order_simulation_v1",
        state_available: true,
        unavailable_reason: None,
        block: status.selected_block_number,
        block_hash: status.selected_block_hash,
        parent_block: parent_status
            .as_ref()
            .map(|status| status.selected_block_number),
        parent_block_hash: parent_status.and_then(|status| status.selected_block_hash),
        state_source: "chain_server_live_tx_simulator",
        report: Some(with_live_chain_sim_evidence(
            report,
            submitted_block,
            execution_block,
            Some(status.selected_block_number),
            status.selected_block_hash,
        )),
    })
}

async fn simulate_order_with_chain(
    simulator: Arc<tx_simulator::TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: PoolSnapshot,
    block: u64,
    skip_uneconomic_sell: bool,
    mut chain: UnsignedTxChainSimulation,
) -> Result<ExecutionReport> {
    match intent.side {
        OrderSide::Buy => {
            simulate_buy_with_chain(
                simulator,
                tx_processor,
                order_id,
                intent,
                &pool,
                block,
                chain,
            )
            .await
        }
        OrderSide::Sell => {
            simulate_sell_with_chain(
                simulator,
                tx_processor,
                order_id,
                intent,
                &pool,
                block,
                skip_uneconomic_sell,
                &mut chain,
            )
            .await
        }
    }
}

async fn process_unsigned_sequence_with_chain(
    tx_processor: Arc<TxProcessor>,
    transactions: Vec<tx_simulator::UnsignedTransaction>,
    block: u64,
    stop_on_revert: bool,
    mut chain: UnsignedTxChainSimulation,
) -> Result<Vec<tx_processor::ProcessedTransaction>> {
    let mut processed = Vec::with_capacity(transactions.len());
    for (idx, mut tx) in transactions.into_iter().enumerate() {
        let base_fee = chain.block_base_fee();
        if tx.gas_price.is_none() && tx.max_fee_per_gas.is_none() {
            tx.max_fee_per_gas = base_fee.or(Some(1));
            tx.max_priority_fee_per_gas.get_or_insert(0);
        }
        let simulation = chain.step_with_trace(tx.clone()).await?;
        let success = simulation.success;
        let processed_tx = tx_processor
            .process_transaction_from_simulation_result(&tx, &simulation, block, idx as u64)
            .await?;
        processed.push(processed_tx);
        if stop_on_revert && !success {
            break;
        }
    }
    Ok(processed)
}

fn pool_buy_sell_config_from_wire(
    config: LivePoolBuySellSimulationConfig,
    block: u64,
) -> PoolBuySellParameters {
    PoolBuySellParameters {
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        test_amount: config.test_amount,
        buyer_address: config.buyer_address,
        prior_txs: config.prior_txs,
        block_number: Some(block),
        block_header: None,
        slippage_tolerance: if config.slippage_tolerance > 0.0 {
            config.slippage_tolerance
        } else {
            5.0
        },
        gas_price: config.gas_price,
        max_fee_per_gas: config.max_fee_per_gas,
        max_priority_fee_per_gas: config.max_priority_fee_per_gas,
        buy_gas_limit: config.buy_gas_limit,
        approve_gas_limit: config.approve_gas_limit,
        sell_gas_limit: config.sell_gas_limit,
        weth_address: config.weth_address,
        denom_address: config.denom_address,
        denom_decimals: config.denom_decimals,
        block_delay: config.block_delay,
        token_decimals: config.token_decimals,
        uniswap_v4_config: config.uniswap_v4_config,
    }
}

async fn simulate_buy_with_chain(
    simulator: Arc<tx_simulator::TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
    mut chain: UnsignedTxChainSimulation,
) -> Result<ExecutionReport> {
    let eth_amount = intent.amount.raw;
    let params = match pool_simulation_parameters_live(
        &mut chain,
        pool,
        intent.token_address,
        eth_amount,
        block,
    ) {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for live chain simulation: {error}"),
                block,
            ));
        }
    };
    let token_decimals = params.token_decimals;
    let result: BuySwapResult =
        match simulate_buy_swap_with_params_and_chain(simulator, tx_processor, params, chain).await
        {
            Ok(result) => result,
            Err(error) => {
                return Ok(failed_report_at(
                    order_id,
                    format!("live chain buy simulation failed: {error}"),
                    block,
                ));
            }
        };
    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or_else(|| "buy simulation failed".to_string()),
            block,
            Some(result.buy_transaction.fees.gas_used),
            Some(result.buy_transaction.fees.tx_fee),
        ));
    }

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(intent.amount),
        token_amount: Some(Amount {
            raw: result.tokens_received,
            decimals: token_decimals,
        }),
        gas_used: Some(result.buy_transaction.fees.gas_used),
        gas_cost: Some(gas_cost_amount(result.buy_transaction.fees.tx_fee)),
        mined_evidence: None,
        error: None,
    })
}

async fn simulate_sell_with_chain(
    simulator: Arc<tx_simulator::TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
    skip_uneconomic_sell: bool,
    chain: &mut UnsignedTxChainSimulation,
) -> Result<ExecutionReport> {
    let tokens_to_sell = intent.amount.raw;
    if tokens_to_sell.is_zero() {
        return Ok(failed_report_at(
            order_id,
            "zero token amount; nothing to sell",
            block,
        ));
    }
    let params =
        match pool_simulation_parameters_live(chain, pool, intent.token_address, U256::ZERO, block)
        {
            Ok(params) => params,
            Err(error) => {
                return Ok(failed_report_at(
                    order_id,
                    format!("invalid pool parameters for live chain simulation: {error}"),
                    block,
                ));
            }
        };
    let denom_decimals = params.denom_decimals;
    let result: SellSwapResult = match simulate_sell_swap_with_params_and_chain(
        simulator,
        tx_processor,
        params,
        tokens_to_sell,
        chain.clone(),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live chain sell simulation failed: {error}"),
                block,
            ));
        }
    };
    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or_else(|| "sell simulation failed".to_string()),
            block,
            Some(result.gas_used),
            Some(result.gas_cost),
        ));
    }

    if skip_uneconomic_sell {
        if let Some(reason) =
            uneconomic_sell_cancellation_reason(pool, result.denom_received, result.gas_cost)
        {
            return Ok(cancelled_report_at(order_id, reason, block));
        }
    }

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(Amount {
            raw: result.denom_received,
            decimals: denom_decimals,
        }),
        token_amount: None,
        gas_used: Some(result.gas_used),
        gas_cost: Some(gas_cost_amount(result.gas_cost)),
        mined_evidence: None,
        error: None,
    })
}

fn pool_simulation_parameters_live(
    chain: &mut UnsignedTxChainSimulation,
    pool: &PoolSnapshot,
    token_address: Address,
    amount: U256,
    block: u64,
) -> std::result::Result<PoolBuySellParameters, String> {
    let pool_contract_address = parse_pool_address(&pool.address)
        .map_err(|error| format!("invalid pool address for chain simulation: {error}"))?;
    let pool_type = pool_type_for_pool(pool).ok_or_else(|| {
        format!(
            "protocol {:?} not supported by chain simulator",
            pool.protocol
        )
    })?;
    let token_decimals = match pool.token_decimals {
        Some(decimals) => decimals,
        None => query_erc20_decimals_on_live_chain(chain, token_address)?,
    };
    let denom_address = pool
        .denom_address
        .ok_or_else(|| format!("pool {} has no denomination address", pool.address))?;
    let denom_decimals = denom_decimals_live(chain, denom_address)?;

    let mut params = PoolBuySellParameters::new(token_address, pool_contract_address, pool_type)
        .with_test_amount(amount)
        .with_block(block)
        .with_denom_address(denom_address)
        .with_denom_decimals(denom_decimals)
        .with_token_decimals(token_decimals);

    if let Some(v4) = &pool.uniswap_v4 {
        params = params.with_uniswap_v4_config(TxUniswapV4PoolConfig {
            pool_manager: v4.pool_manager,
            pool_id: v4.pool_id,
            currency0: v4.currency0,
            currency1: v4.currency1,
            fee: v4.fee,
            tick_spacing: v4.tick_spacing,
            hooks: v4.hooks,
            hook_data: Vec::new(),
        });
    }

    Ok(params)
}

fn pool_type_for_pool(pool: &PoolSnapshot) -> Option<PoolType> {
    match &pool.protocol {
        PoolProtocol::UniswapV2 => Some(PoolType::UniswapV2),
        PoolProtocol::UniswapV3 => Some(PoolType::UniswapV3 {
            fee_tier: pool.fee_tier.unwrap_or(3000),
        }),
        PoolProtocol::UniswapV4 => Some(PoolType::UniswapV4),
        PoolProtocol::PancakeSwapV2 => Some(PoolType::PancakeSwapV2),
        PoolProtocol::Unknown(name) => match name.as_str() {
            "sushi" | "sushiswap" => Some(PoolType::SushiSwap),
            _ => None,
        },
    }
}

fn denom_decimals_live(
    chain: &mut UnsignedTxChainSimulation,
    denom_address: Address,
) -> std::result::Result<u8, String> {
    if denom_address.is_zero() || denom_address == WETH_ADDRESS || denom_address == DAI_ADDRESS {
        return Ok(18);
    }
    if denom_address == USDC_ADDRESS || denom_address == USDT_ADDRESS {
        return Ok(6);
    }
    query_erc20_decimals_on_live_chain(chain, denom_address)
}

fn query_erc20_decimals_on_live_chain(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
) -> std::result::Result<u8, String> {
    let result = chain
        .simulate_view_call(token_address, Bytes::from_static(&ERC20_DECIMALS_SELECTOR))
        .map_err(|error| format!("token decimals live view simulation failed: {error}"))?;
    if !result.success {
        return Err("token decimals live view simulation reverted".to_string());
    }
    if result.output.len() < 32 {
        return Err(format!(
            "token decimals live view simulation returned short output: {} bytes",
            result.output.len()
        ));
    }
    Ok(result.decode_uint8())
}

fn parse_pool_address(pool_id: &PoolAddress) -> std::result::Result<Address, String> {
    let s = pool_id.as_str();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 {
        return Err(s.to_string());
    }
    let pool_part = parts[1];
    let addr_str = pool_part.split('#').next().unwrap_or(pool_part);
    addr_str.parse::<Address>().map_err(|_| s.to_string())
}

fn failed_report_at(order_id: OrderId, reason: impl Into<String>, block: u64) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Failed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

fn failed_report_at_with_gas(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
    gas_used: Option<u64>,
    gas_cost: Option<U256>,
) -> ExecutionReport {
    let mut report = failed_report_at(order_id, reason, block);
    report.gas_used = gas_used;
    report.gas_cost = gas_cost.map(gas_cost_amount);
    report
}

fn cancelled_report_at(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Cancelled,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

fn gas_cost_amount(raw: U256) -> Amount {
    Amount { raw, decimals: 18 }
}

fn with_live_chain_sim_evidence(
    mut report: ExecutionReport,
    submitted_block: u64,
    expected_confirmation_block: u64,
    simulation_block: Option<u64>,
    block_hash: Option<B256>,
) -> ExecutionReport {
    let receipt_block = report.block_number;
    let mut evidence = report.mined_evidence.unwrap_or_default();
    evidence.submitted_block_number = Some(submitted_block);
    evidence.expected_confirmation_block = Some(expected_confirmation_block);
    evidence.receipt_block_number = receipt_block;
    evidence.simulation_block_number = simulation_block;
    evidence.confirmation_lag_blocks =
        receipt_block.map(|block| block as i64 - expected_confirmation_block as i64);
    evidence.block_hash = block_hash;
    evidence.receipt_status = Some("live_backtest_chain_sim".to_string());
    report.mined_evidence = Some(evidence);
    report
}

fn uneconomic_sell_cancellation_reason(
    pool: &PoolSnapshot,
    denom_received: U256,
    gas_cost: U256,
) -> Option<String> {
    if !pool_has_eth_like_denom(pool) || denom_received > gas_cost {
        return None;
    }
    Some(format!(
        "uneconomic sell: simulated WETH proceeds {denom_received} wei <= gas cost {gas_cost} wei"
    ))
}

fn pool_has_eth_like_denom(pool: &PoolSnapshot) -> bool {
    if matches!(pool.denom_address, Some(address) if address.is_zero() || address == WETH_ADDRESS) {
        return true;
    }
    pool.denom_symbol
        .as_deref()
        .map(str::trim)
        .map(|symbol| {
            let symbol = symbol.to_ascii_uppercase();
            symbol == "ETH" || symbol == "WETH"
        })
        .unwrap_or(false)
}

fn unavailable_order_response(
    block: u64,
    reason: impl Into<String>,
) -> LiveOrderSimulationResponse {
    LiveOrderSimulationResponse {
        schema: "eth_live_order_simulation_v1",
        state_available: false,
        unavailable_reason: Some(reason.into()),
        block,
        block_hash: None,
        parent_block: None,
        parent_block_hash: None,
        state_source: "chain_server_live_tx_simulator",
        report: None,
    }
}

fn unavailable_order_response_with_status(
    status: LiveStateStatus,
    reason: impl Into<String>,
) -> LiveOrderSimulationResponse {
    LiveOrderSimulationResponse {
        schema: "eth_live_order_simulation_v1",
        state_available: false,
        unavailable_reason: Some(reason.into()),
        block: status.selected_block_number,
        block_hash: status.selected_block_hash,
        parent_block: None,
        parent_block_hash: None,
        state_source: "chain_server_live_tx_simulator",
        report: None,
    }
}

fn unavailable_order_response_with_parent(
    block: u64,
    parent_status: Option<LiveStateStatus>,
    reason: impl Into<String>,
) -> LiveOrderSimulationResponse {
    LiveOrderSimulationResponse {
        schema: "eth_live_order_simulation_v1",
        state_available: false,
        unavailable_reason: Some(reason.into()),
        block,
        block_hash: None,
        parent_block: parent_status
            .as_ref()
            .map(|status| status.selected_block_number),
        parent_block_hash: parent_status.and_then(|status| status.selected_block_hash),
        state_source: "chain_server_live_tx_simulator",
        report: None,
    }
}
