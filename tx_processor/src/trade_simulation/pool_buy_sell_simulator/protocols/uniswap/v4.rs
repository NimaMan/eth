use std::sync::Arc;

use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result, WrapErr};
use tx_simulator::{
    tx_builders::{
        permit2::build_permit2_approve_tx,
        uniswap_v4::{
            build_token_approval_tx, build_universal_router_v4_exact_input_single_tx,
            build_weth_deposit_tx, infer_orientation_from_input, infer_orientation_from_output,
            UniswapV4PoolKey as BuilderV4PoolKey, UniversalRouterV4ExactInputSingleRequest,
            UniversalRouterV4InputPayment,
        },
    },
    FullSimulationResult, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};

use crate::processed_tx_builder::UnsignedTxBuilder;
use crate::trade_simulation::pool_buy_sell_simulator::common::balance_deltas::{
    extract_denom_received_from_processed_transaction, extract_token_balance_delta,
    extract_tokens_received_from_processed_transaction,
};
use crate::trade_simulation::pool_buy_sell_simulator::common::block_header::block_header_hint;
use crate::trade_simulation::pool_buy_sell_simulator::common::buyer_setup::{
    ensure_buyer_eth_for_probe, prepare_buyer_account,
};
use crate::trade_simulation::pool_buy_sell_simulator::common::failure::{
    format_failure_with_full_trace, format_failure_with_revert,
};
use crate::trade_simulation::pool_buy_sell_simulator::common::fees::{
    apply_fee_policy, normalize_prior_fees_with_header,
};
use crate::trade_simulation::pool_buy_sell_simulator::common::replay_funding::ensure_replay_sender_can_pay;
use crate::trade_simulation::pool_buy_sell_simulator::common::results::create_failed_result;
use crate::trade_simulation::types::{
    PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};
use crate::tx_processor::TxProcessor;

const UNIVERSAL_ROUTER_V4: Address = address!("66a9893cC07D91D95644AEDD05D03f95e1dBA8Af");
const PERMIT2: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");
const PERMIT2_EXPIRATION: u64 = (1_u64 << 48) - 1;
const ERC20_TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

pub(in crate::trade_simulation::pool_buy_sell_simulator) async fn check_can_buy_sell_uniswap_v4(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
) -> Result<PoolBuySellSimulationResult> {
    if config.block_delay > 0 {
        return Err(eyre!(
            "Uniswap V4 pool simulation currently does not support block delays"
        ));
    }
    if config.token_address.is_zero() {
        return Err(eyre!("Uniswap V4 target token must be an ERC20 address"));
    }

    let block_number = config
        .block_number
        .unwrap_or(simulator.latest_historical_context_block_number()?);
    let header_hint = block_header_hint(&config, block_number)?;
    let header = match header_hint.clone() {
        Some(header) => header,
        None => {
            simulator
                .block_context_loader()
                .load_block_header(block_number, None)
                .await?
        }
    };
    let base_fee = header.header().base_fee_per_gas.map(|fee| fee as u128);
    let chain = match header_hint {
        Some(header) => {
            simulator
                .start_simulation_chain_with_header(block_number, header)
                .await?
        }
        None => simulator.start_simulation_chain(Some(block_number)).await?,
    };

    check_can_buy_sell_uniswap_v4_with_prepared_chain(
        tx_processor,
        config,
        block_number,
        base_fee,
        chain,
    )
    .await
}

pub(in crate::trade_simulation::pool_buy_sell_simulator) async fn check_can_buy_sell_uniswap_v4_with_chain(
    tx_processor: Arc<TxProcessor>,
    mut config: PoolBuySellParameters,
    chain: UnsignedTxChainSimulation,
) -> Result<PoolBuySellSimulationResult> {
    config.prior_txs.clear();
    if config.block_delay > 0 {
        return Err(eyre!(
            "Uniswap V4 pool simulation currently does not support block delays"
        ));
    }
    if config.token_address.is_zero() {
        return Err(eyre!("Uniswap V4 target token must be an ERC20 address"));
    }

    let block_number = config
        .block_number
        .unwrap_or_else(|| chain.current_state().block_number);
    let base_fee = match block_header_hint(&config, block_number)? {
        Some(header) => header.header().base_fee_per_gas.map(|fee| fee as u128),
        None => chain.block_base_fee(),
    };

    check_can_buy_sell_uniswap_v4_with_prepared_chain(
        tx_processor,
        config,
        block_number,
        base_fee,
        chain,
    )
    .await
}

async fn check_can_buy_sell_uniswap_v4_with_prepared_chain(
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
    block_number: u64,
    base_fee: Option<u128>,
    mut chain: UnsignedTxChainSimulation,
) -> Result<PoolBuySellSimulationResult> {
    let v4_cfg = config
        .uniswap_v4_config
        .clone()
        .ok_or_else(|| eyre!("Uniswap V4 configuration must be provided"))?;

    if !chain.account_has_code(v4_cfg.pool_manager)? {
        return Ok(create_failed_result(
            config,
            block_number,
            Vec::new(),
            None,
            None,
            None,
            format!(
                "Uniswap V4 PoolManager {:#x} has no bytecode at block {}",
                v4_cfg.pool_manager, block_number
            ),
            false,
            false,
            false,
        ));
    }

    let pool_key = BuilderV4PoolKey {
        currency0: v4_cfg.currency0,
        currency1: v4_cfg.currency1,
        fee: v4_cfg.fee,
        tick_spacing: v4_cfg.tick_spacing,
        hooks: v4_cfg.hooks,
    };

    let buy_orientation = infer_orientation_from_output(&pool_key, config.token_address)?;
    if !currency_matches_denom(
        buy_orientation.input_currency,
        config.denom_address,
        config.weth_address,
    ) {
        return Err(eyre!(
            "Uniswap V4 buy input currency {:#x} does not match configured denom {:#x}",
            buy_orientation.input_currency,
            config.denom_address
        ));
    }

    let mut prior_tx_results = Vec::with_capacity(config.prior_txs.len() + 5);
    replay_prior_transactions(
        &mut chain,
        tx_processor.clone(),
        &config,
        base_fee,
        block_number,
        &mut prior_tx_results,
    )
    .await?;

    if let Some(failure) = prepare_v4_buy_input(
        &mut chain,
        tx_processor.clone(),
        &config,
        base_fee,
        block_number,
        buy_orientation.input_currency,
        &mut prior_tx_results,
    )
    .await?
    {
        return Ok(failure);
    }

    let prior_step_count = prior_tx_results.len() as u64;
    let mut buy_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: UNIVERSAL_ROUTER_V4,
            caller: config.buyer_address,
            pool_key: pool_key.clone(),
            token_in: buy_orientation.input_currency,
            token_out: buy_orientation.output_currency,
            amount_in: config.test_amount,
            min_amount_out: U256::ZERO,
            deadline: U256::from(u64::MAX),
            hook_data: v4_cfg.hook_data.clone(),
            input_payment: payment_for_input(buy_orientation.input_currency, config.weth_address),
        },
    )?;
    buy_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut buy_tx, &config, base_fee);
    let (buy_result, buy_processed) = simulate_and_process(
        &mut chain,
        tx_processor.clone(),
        buy_tx.clone(),
        block_number,
        prior_step_count,
        "v4_buy_universal_router",
    )
    .await
    .wrap_err("while executing Uniswap V4 buy through Universal Router")?;

    if !buy_result.success {
        let failure_message = format_failure_with_full_trace(
            "Universal Router V4 buy transaction failed",
            &buy_result,
        );

        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            None,
            None,
            failure_message,
            false,
            false,
            false,
        ));
    }

    let tokens_received = extract_tokens_received_from_processed_transaction(
        &buy_processed,
        config.buyer_address,
        config.token_address,
        config.token_decimals,
    );
    if tokens_received.is_zero() {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            None,
            None,
            "Universal Router V4 buy succeeded but buyer received zero target tokens".to_string(),
            false,
            false,
            false,
        ));
    }

    let mut token_erc20_approve = build_token_approval_tx(
        config.buyer_address,
        config.token_address,
        PERMIT2,
        U256::MAX,
    );
    token_erc20_approve.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut token_erc20_approve, &config, base_fee);
    let (token_erc20_approve_result, token_erc20_approve_processed) = simulate_and_process(
        &mut chain,
        tx_processor.clone(),
        token_erc20_approve.clone(),
        block_number,
        prior_step_count + 1,
        "v4_token_approve_permit2",
    )
    .await
    .wrap_err("while approving V4 output token for Permit2")?;
    if !token_erc20_approve_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(token_erc20_approve_processed),
            None,
            format_failure_with_revert(
                "Token approval for Permit2 failed",
                token_erc20_approve_result.revert_reason.as_deref(),
            ),
            true,
            false,
            false,
        ));
    }
    prior_tx_results.push(token_erc20_approve_processed);

    let mut token_permit2_approve = build_permit2_approve_tx(
        config.buyer_address,
        PERMIT2,
        config.token_address,
        UNIVERSAL_ROUTER_V4,
        permit2_amount(tokens_received)?,
        PERMIT2_EXPIRATION,
    )?;
    token_permit2_approve.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut token_permit2_approve, &config, base_fee);
    let (token_permit2_approve_result, approve_processed) = simulate_and_process(
        &mut chain,
        tx_processor.clone(),
        token_permit2_approve.clone(),
        block_number,
        prior_step_count + 2,
        "v4_permit2_approve_universal_router",
    )
    .await
    .wrap_err("while approving Universal Router in Permit2")?;
    if !token_permit2_approve_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(approve_processed),
            None,
            format_failure_with_revert(
                "Permit2 approval for Universal Router failed",
                token_permit2_approve_result.revert_reason.as_deref(),
            ),
            true,
            false,
            false,
        ));
    }

    let sell_orientation = infer_orientation_from_input(&pool_key, config.token_address)?;
    if !currency_matches_denom(
        sell_orientation.output_currency,
        config.denom_address,
        config.weth_address,
    ) {
        return Err(eyre!(
            "Uniswap V4 sell output currency {:#x} does not match configured denom {:#x}",
            sell_orientation.output_currency,
            config.denom_address
        ));
    }

    let mut sell_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: UNIVERSAL_ROUTER_V4,
            caller: config.buyer_address,
            pool_key,
            token_in: config.token_address,
            token_out: sell_orientation.output_currency,
            amount_in: tokens_received,
            min_amount_out: U256::ZERO,
            deadline: U256::from(u64::MAX),
            hook_data: v4_cfg.hook_data,
            input_payment: UniversalRouterV4InputPayment::Permit2User,
        },
    )?;
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_fee_policy(&mut sell_tx, &config, base_fee);
    let (sell_result, sell_processed) = simulate_and_process(
        &mut chain,
        tx_processor.clone(),
        sell_tx.clone(),
        block_number,
        prior_step_count + 3,
        "v4_sell_universal_router",
    )
    .await
    .wrap_err("while executing Uniswap V4 sell through Universal Router")?;
    if !sell_result.success {
        let failure_message = format_failure_with_full_trace(
            "Universal Router V4 sell transaction failed",
            &sell_result,
        );

        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(approve_processed),
            Some(sell_processed),
            failure_message,
            true,
            true,
            false,
        ));
    }

    let buy_tax = calculate_buy_tax_from_processed_transaction(
        &buy_processed,
        config.pool_address,
        config.buyer_address,
        config.token_address,
    );
    let sell_tax = calculate_sell_tax_from_processed_transaction(
        &sell_processed,
        config.pool_address,
        config.buyer_address,
    );
    let denom_spent = denom_spent_from_buy(&buy_processed, &config, buy_orientation.input_currency);
    let denom_received = extract_denom_received_from_processed_transaction(
        &sell_processed,
        config.buyer_address,
        config.denom_address,
    )
    .unwrap_or(U256::ZERO);

    Ok(PoolBuySellSimulationResult {
        pool_type: PoolType::UniswapV4,
        pool_address: config.pool_address,
        token_address: config.token_address,
        can_buy: true,
        can_approve: true,
        can_sell: true,
        is_tradeable: true,
        buy_tax_percent: buy_tax.as_percentage().unwrap_or(0.0),
        sell_tax_percent: sell_tax.as_percentage().unwrap_or(0.0),
        tokens_received,
        denom_spent,
        denom_received,
        buy_transaction: buy_processed,
        sell_transaction: sell_processed,
        approve_transaction: approve_processed,
        prior_transactions: prior_tx_results,
        failure_reason: None,
        block_number,
    })
}

async fn replay_prior_transactions(
    chain: &mut UnsignedTxChainSimulation,
    tx_processor: Arc<TxProcessor>,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
    block_number: u64,
    prior_tx_results: &mut Vec<ProcessedTransaction>,
) -> Result<()> {
    for (idx, prior_tx) in config.prior_txs.iter().enumerate() {
        let mut setup_call = UnsignedTxBuilder::build_unsigned_from_processed_tx(prior_tx);
        setup_call.nonce = Some(prior_tx.nonce);
        if setup_call.gas.is_none() {
            setup_call.gas = if prior_tx.fees.gas_limit > 0 {
                Some(prior_tx.fees.gas_limit)
            } else if prior_tx.fees.gas_used > 0 {
                Some(prior_tx.fees.gas_used)
            } else {
                None
            };
        }
        normalize_prior_fees_with_header(base_fee, prior_tx, &mut setup_call);

        let has_explicit_fee = setup_call.gas_price.is_some()
            || setup_call.max_fee_per_gas.is_some()
            || setup_call.max_priority_fee_per_gas.is_some();
        if !has_explicit_fee {
            apply_fee_policy(&mut setup_call, config, base_fee);
        }

        let prior_hash = format!("{:#x}", prior_tx.hash);
        let prior_nonce = prior_tx.nonce;
        chain.set_account_nonce_for_replay(prior_tx.from_address, prior_nonce)?;
        ensure_replay_sender_can_pay(chain, &setup_call)?;
        let (setup_result, processed) = simulate_and_process(
            chain,
            tx_processor.clone(),
            setup_call,
            block_number,
            idx as u64,
            "v4_prior_replay",
        )
        .await
        .map_err(|err| {
            eyre!("while replaying prior tx {prior_hash} (index {idx}, nonce {prior_nonce}): {err}")
        })?;
        prior_tx_results.push(processed);

        if !setup_result.success {
            return Err(eyre!(
                "setup transaction replay failed tx={} mined_block={} tx_index={} nonce={} from={:#x} simulation_base_block={}",
                prior_hash,
                prior_tx.block_number,
                prior_tx.tx_index,
                prior_nonce,
                prior_tx.from_address,
                block_number
            ));
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn prepare_v4_buy_input(
    chain: &mut UnsignedTxChainSimulation,
    tx_processor: Arc<TxProcessor>,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
    block_number: u64,
    input_currency: Address,
    prior_tx_results: &mut Vec<ProcessedTransaction>,
) -> Result<Option<PoolBuySellSimulationResult>> {
    ensure_buyer_eth_for_probe(chain, config, block_number)?;

    if input_currency.is_zero() {
        return Ok(None);
    }

    if input_currency == config.weth_address {
        let mut deposit_tx = build_weth_deposit_tx(
            config.buyer_address,
            config.weth_address,
            config.test_amount,
        );
        deposit_tx.gas = Some(config.buy_gas_limit);
        apply_fee_policy(&mut deposit_tx, config, base_fee);
        let (deposit_result, deposit_processed) = simulate_and_process(
            chain,
            tx_processor.clone(),
            deposit_tx,
            block_number,
            prior_tx_results.len() as u64,
            "v4_weth_deposit",
        )
        .await
        .wrap_err("while depositing ETH into WETH for V4 input")?;
        prior_tx_results.push(deposit_processed);
        if !deposit_result.success {
            return Ok(Some(create_failed_result(
                config.clone(),
                block_number,
                prior_tx_results.clone(),
                None,
                None,
                None,
                format_failure_with_revert(
                    "WETH deposit failed for Uniswap V4 input",
                    deposit_result.revert_reason.as_deref(),
                ),
                false,
                false,
                false,
            )));
        }

        let mut transfer_tx = build_token_transfer_tx(
            config.buyer_address,
            input_currency,
            UNIVERSAL_ROUTER_V4,
            config.test_amount,
        );
        transfer_tx.gas = Some(config.approve_gas_limit);
        apply_fee_policy(&mut transfer_tx, config, base_fee);
        let (transfer_result, transfer_processed) = simulate_and_process(
            chain,
            tx_processor.clone(),
            transfer_tx,
            block_number,
            prior_tx_results.len() as u64,
            "v4_input_transfer_to_universal_router",
        )
        .await
        .wrap_err("while pre-funding Universal Router with WETH for V4 input")?;
        prior_tx_results.push(transfer_processed);
        if !transfer_result.success {
            return Ok(Some(create_failed_result(
                config.clone(),
                block_number,
                prior_tx_results.clone(),
                None,
                None,
                None,
                format_failure_with_revert(
                    "WETH transfer to Universal Router failed for Uniswap V4 input",
                    transfer_result.revert_reason.as_deref(),
                ),
                false,
                false,
                false,
            )));
        }

        return Ok(None);
    } else if let Some(failure) = prepare_buyer_account(
        chain,
        config,
        base_fee,
        tx_processor.clone(),
        block_number,
        prior_tx_results,
    )
    .await?
    {
        return Ok(Some(failure));
    }

    let mut erc20_approve =
        build_token_approval_tx(config.buyer_address, input_currency, PERMIT2, U256::MAX);
    erc20_approve.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut erc20_approve, config, base_fee);
    let (erc20_result, erc20_processed) = simulate_and_process(
        chain,
        tx_processor.clone(),
        erc20_approve,
        block_number,
        prior_tx_results.len() as u64,
        "v4_input_approve_permit2",
    )
    .await
    .wrap_err("while approving V4 input token for Permit2")?;
    prior_tx_results.push(erc20_processed);
    if !erc20_result.success {
        return Ok(Some(create_failed_result(
            config.clone(),
            block_number,
            prior_tx_results.clone(),
            None,
            None,
            None,
            format_failure_with_revert(
                "V4 input token approval for Permit2 failed",
                erc20_result.revert_reason.as_deref(),
            ),
            false,
            false,
            false,
        )));
    }

    let mut permit2_approve = build_permit2_approve_tx(
        config.buyer_address,
        PERMIT2,
        input_currency,
        UNIVERSAL_ROUTER_V4,
        permit2_amount(config.test_amount)?,
        PERMIT2_EXPIRATION,
    )?;
    permit2_approve.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut permit2_approve, config, base_fee);
    let (permit2_result, permit2_processed) = simulate_and_process(
        chain,
        tx_processor,
        permit2_approve,
        block_number,
        prior_tx_results.len() as u64,
        "v4_input_permit2_approve_universal_router",
    )
    .await
    .wrap_err("while approving Universal Router for V4 input in Permit2")?;
    prior_tx_results.push(permit2_processed);
    if !permit2_result.success {
        return Ok(Some(create_failed_result(
            config.clone(),
            block_number,
            prior_tx_results.clone(),
            None,
            None,
            None,
            format_failure_with_revert(
                "V4 input Permit2 approval for Universal Router failed",
                permit2_result.revert_reason.as_deref(),
            ),
            false,
            false,
            false,
        )));
    }
    Ok(None)
}

async fn simulate_and_process(
    chain: &mut UnsignedTxChainSimulation,
    tx_processor: Arc<TxProcessor>,
    tx: UnsignedTransaction,
    block_number: u64,
    tx_index: u64,
    step: &'static str,
) -> Result<(FullSimulationResult, ProcessedTransaction)> {
    let result = chain
        .step_with_trace(tx.clone())
        .await
        .map_err(|err| {
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step,
                block = block_number,
                error = %err,
                "failed Uniswap V4 simulation step"
            );
            err
        })
        .wrap_err_with(|| {
            format!("while executing Uniswap V4 simulation step {step} at block {block_number}")
        })?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&tx, &result, block_number, tx_index)
        .await
        .wrap_err_with(|| {
            format!("while processing Uniswap V4 simulation step {step} at block {block_number}")
        })?;
    Ok((result, processed))
}

fn payment_for_input(
    input_currency: Address,
    weth_address: Address,
) -> UniversalRouterV4InputPayment {
    if input_currency.is_zero() {
        UniversalRouterV4InputPayment::NativeEth
    } else if input_currency == weth_address {
        UniversalRouterV4InputPayment::RouterBalance
    } else {
        UniversalRouterV4InputPayment::Permit2User
    }
}

fn currency_matches_denom(currency: Address, denom: Address, weth: Address) -> bool {
    currency == denom || (currency.is_zero() && (denom.is_zero() || denom == weth))
}

fn permit2_amount(amount: U256) -> Result<U256> {
    let max = (U256::from(1_u8) << 160) - U256::from(1_u8);
    if amount > max {
        return Err(eyre!("Permit2 allowance amount must fit uint160"));
    }
    Ok(amount)
}

fn denom_spent_from_buy(
    buy_processed: &ProcessedTransaction,
    config: &PoolBuySellParameters,
    input_currency: Address,
) -> U256 {
    if input_currency.is_zero() {
        return config.test_amount;
    }
    if input_currency == config.weth_address {
        return config.test_amount;
    }

    let delta = extract_token_balance_delta(buy_processed, config.buyer_address, input_currency);
    if delta.is_negative() {
        delta.unsigned_abs()
    } else {
        U256::ZERO
    }
}

fn build_token_transfer_tx(
    owner: Address,
    token: Address,
    recipient: Address,
    amount: U256,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&ERC20_TRANSFER_SELECTOR);
    data.extend_from_slice(&pad_address(recipient));
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(120_000),
        value: Some(U256::ZERO),
        data: Some(data.into()),
        ..Default::default()
    }
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut word = [0_u8; 32];
    word[12..].copy_from_slice(address.as_slice());
    word
}
