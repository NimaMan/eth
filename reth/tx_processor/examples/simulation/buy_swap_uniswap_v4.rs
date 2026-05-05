use alloy_primitives::{Address, I256, U256};
use eyre::Result;
use reth_chain_query::common_addresses::uniswap_v4_pools;
use reth_chain_query::to_checksum_address;
use reth_chain_query::tx_builders::uniswap_v4::{
    build_baygus_executor_deploy_tx, build_baygus_executor_multihop_tx,
    build_baygus_executor_single_hop_exact_input_call, build_router_deploy_tx,
    build_swap_exact_input_single_tx, build_token_approval_tx, build_weth_deposit_tx,
    compute_contract_address, infer_orientation_from_output, UniswapV4BaygusSingleHopRequest,
    UniswapV4PoolKey,
};
use reth_provider::AccountReader;
use std::sync::Arc;
use tx_processor::simulator::types::PoolBuySellParameters;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::UnsignedTransaction;
use tx_simulator::TxSimulator;

const TEST_AMOUNT_WEI: u128 = 10_000_000_000_000_000; // 0.01 ETH

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦄 Uniswap V4 Router Deploy + Buy Simulation");
    println!("===========================================\n");

    let pools = uniswap_v4_pools();
    if pools.is_empty() {
        return Err(eyre::eyre!(
            "No canonical Uniswap V4 pools configured in dex_token_denom_pairs"
        ));
    }

    let pool = &pools[0];
    println!("Target pool : {} / {}", pool.symbol, pool.denom_symbol);
    println!(
        "Pool details: id {:#x}, fee {} (tick spacing {}), hook {}",
        pool.pool_id, pool.fee, pool.tick_spacing, pool.hooks
    );

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    println!("Reth datadir: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    let default_cfg = PoolBuySellParameters::default();
    let buyer_address = default_cfg.buyer_address;
    let weth_address = default_cfg.weth_address;

    let resolved_block = match pool.block_hint {
        Some(block) => {
            println!("Using block hint : {}", block);
            block
        }
        None => {
            let latest = simulator.get_latest_block()?;
            println!("Latest snapshot : {}", latest);
            latest
        }
    };

    let provider = simulator.provider_factory().provider()?;
    let deployer_nonce = provider
        .basic_account(&buyer_address)?
        .map(|acc| acc.nonce)
        .unwrap_or(0);
    let router_address = compute_contract_address(buyer_address, deployer_nonce);
    println!("Router address  : {router_address}");

    let mut chain = simulator
        .start_simulation_chain(Some(resolved_block))
        .await?;

    let mut step_index = 0u64;

    // 1) Deploy the Baygus executor (default) unless the legacy minimal router is explicitly forced.
    let force_minimal_router = std::env::var("UNISWAP_V4_USE_MINIMAL_ROUTER")
        .map(|flag| {
            let lowered = flag.trim().to_ascii_lowercase();
            lowered == "1" || lowered == "true" || lowered == "yes"
        })
        .unwrap_or(false);
    let use_baygus_executor = pool.hooks != Address::ZERO || !force_minimal_router;
    if use_baygus_executor && pool.hooks == Address::ZERO {
        println!(
            "Using Baygus executor on hookless pool (set UNISWAP_V4_USE_MINIMAL_ROUTER=1 to use MinimalV4Router)"
        );
    } else if !use_baygus_executor {
        println!("Using legacy MinimalV4Router because UNISWAP_V4_USE_MINIMAL_ROUTER=1");
    }

    let mut deploy_tx = if use_baygus_executor {
        build_baygus_executor_deploy_tx(buyer_address, pool.pool_manager)
    } else {
        build_router_deploy_tx(buyer_address, pool.pool_manager, weth_address)
    }?;
    apply_simple_gas_policy(&mut deploy_tx);
    let deploy_result = chain.step_with_trace(deploy_tx.clone()).await?;
    let router_code_before = chain.account_has_code(router_address)?;
    let deploy_processed = tx_processor
        .process_transaction_from_simulation_result(
            &deploy_tx,
            &deploy_result,
            resolved_block,
            step_index,
        )
        .await?;
    println!(
        "\n[{}] Router deploy: {} (hash {:#x})",
        step_index + 1,
        status_label(deploy_result.success),
        deploy_processed.hash
    );
    println!("    ⋗ code visible before deposit? {}", router_code_before);
    step_index += 1;

    let pool_key = UniswapV4PoolKey {
        currency0: pool.token_address,
        currency1: pool.denom_address,
        fee: pool.fee,
        tick_spacing: pool.tick_spacing,
        hooks: pool.hooks,
    };

    let buy_orientation = infer_orientation_from_output(&pool_key, pool.token_address)?;
    let test_amount = U256::from(TEST_AMOUNT_WEI);

    // 2) Wrap ETH into WETH if required
    if buy_orientation.input_currency == weth_address {
        let mut deposit_tx = build_weth_deposit_tx(buyer_address, weth_address, test_amount);
        apply_simple_gas_policy(&mut deposit_tx);
        let deposit_result = chain.step_with_trace(deposit_tx.clone()).await?;
        let deposit_processed = tx_processor
            .process_transaction_from_simulation_result(
                &deposit_tx,
                &deposit_result,
                resolved_block,
                step_index,
            )
            .await?;
        println!(
            "[{}] WETH deposit : {} (hash {:#x})",
            step_index + 1,
            status_label(deposit_result.success),
            deposit_processed.hash
        );
        step_index += 1;

        // 3) Approve router to spend WETH
        let mut approve_tx =
            build_token_approval_tx(buyer_address, weth_address, router_address, U256::MAX);
        apply_simple_gas_policy(&mut approve_tx);
        let approve_result = chain.step_with_trace(approve_tx.clone()).await?;
        let approve_processed = tx_processor
            .process_transaction_from_simulation_result(
                &approve_tx,
                &approve_result,
                resolved_block,
                step_index,
            )
            .await?;
        println!(
            "[{}] WETH approval: {} (hash {:#x})",
            step_index + 1,
            status_label(approve_result.success),
            approve_processed.hash
        );
        step_index += 1;
    }

    // 4) Build and execute Baygus executor buy
    let buy_request = UniswapV4BaygusSingleHopRequest {
        pool_key: pool_key.clone(),
        token_in: buy_orientation.input_currency,
        token_out: buy_orientation.output_currency,
        amount_in: test_amount,
        recipient: buyer_address,
        min_output: None,
        hook_adapter: Address::ZERO,
        hook_data: Vec::new(),
        sqrt_price_limit_x96: None,
    };

    let mut buy_tx = if use_baygus_executor {
        let buy_call = build_baygus_executor_single_hop_exact_input_call(&buy_request)?;
        println!(
            "hook adapter: {} sqrt_limit: {}",
            buy_call.params.hops[0].hook_adapter,
            buy_call.params.hops[0].params.sqrt_price_limit_x96
        );
        build_baygus_executor_multihop_tx(
            router_address,
            buyer_address,
            &buy_call.params,
            buy_call.eth_value,
        )?
    } else {
        let min_output = buy_request.min_output.unwrap_or(U256::ZERO);
        build_swap_exact_input_single_tx(
            router_address,
            buyer_address,
            &pool_key,
            buy_orientation.zero_for_one,
            test_amount,
            min_output,
            buyer_address,
            false,
            &[],
        )?
    };
    apply_simple_gas_policy(&mut buy_tx);
    if let Some(data) = buy_tx.data.as_ref() {
        println!(
            "buy calldata ({} bytes): 0x{}",
            data.len(),
            hex::encode(data)
        );
    }

    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(
            &buy_tx,
            &buy_result,
            resolved_block,
            step_index,
        )
        .await?;
    println!(
        "[{}] Router buy    : {} (hash {:#x})",
        step_index + 1,
        status_label(buy_result.success),
        buy_processed.hash
    );
    println!("Call trace tree: {:#?}", buy_result.call_trace);
    if let Some(logs) = &buy_result.struct_logs {
        println!("Struct logs (first 20 of {}):", logs.len());
        for (idx, log) in logs.iter().take(20).enumerate() {
            println!("{idx}: {:?}", log.op);
        }
    }

    let tokens_received =
        extract_positive_amount(&buy_processed, buyer_address, pool.token_address);
    let denom_delta = extract_amount(&buy_processed, buyer_address, pool.denom_address);
    let denom_spent = if denom_delta < I256::ZERO {
        denom_delta.unsigned_abs()
    } else {
        U256::ZERO
    };

    println!("\nSummary");
    println!("-------");
    println!("Simulation block : {}", resolved_block);
    println!("Buy outcome      : {}", status_label(buy_result.success));
    println!(
        "Tokens received  : {} {}",
        format_amount(tokens_received, pool.token_decimals),
        pool.symbol
    );
    println!(
        "Denom change     : -{} {}",
        format_amount(denom_spent, pool.denom_decimals),
        pool.denom_symbol
    );

    if !buy_result.success {
        if let Some(reason) = buy_result.revert_reason.as_deref() {
            println!("\n⚠️ Buy revert reason: {reason}");
        }
    }

    println!("Call trace root: {:?}", buy_result.call_trace);
    if let Some(ctx) = buy_result.revert_context {
        println!(
            "Revert context   : target {} (calldata {} bytes)",
            ctx.target, ctx.calldata_len
        );
    }

    println!(
        "All balance changes: {:?}",
        buy_processed.address_balance_changes
    );

    Ok(())
}

fn format_amount(amount: U256, decimals: u8) -> String {
    if amount.is_zero() {
        return "0".to_string();
    }

    let digits = amount.to_string();
    let decimals = decimals as usize;

    if decimals == 0 {
        return digits;
    }

    if digits.len() <= decimals {
        let padded = format!("{:0>width$}", digits, width = decimals + 1);
        let split = padded.len() - decimals;
        let (whole, frac) = padded.split_at(split);
        let frac_trimmed = frac.trim_end_matches('0');
        if frac_trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{frac_trimmed}")
        }
    } else {
        let split = digits.len() - decimals;
        let (whole, frac) = digits.split_at(split);
        let frac_trimmed = frac.trim_end_matches('0');
        if frac_trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{frac_trimmed}")
        }
    }
}

fn status_label(success: bool) -> &'static str {
    if success {
        "✅ success"
    } else {
        "❌ failed"
    }
}

fn apply_simple_gas_policy(tx: &mut UnsignedTransaction) {
    if tx.gas.is_none() {
        tx.gas = Some(900_000);
    }
    if tx.max_fee_per_gas.is_none() && tx.gas_price.is_none() {
        tx.gas_price = Some(50_000_000_000);
    }
    if tx.max_priority_fee_per_gas.is_none() && tx.gas_price.is_none() {
        tx.max_priority_fee_per_gas = Some(1_000_000_000);
        tx.max_fee_per_gas = Some(50_000_000_000);
    }
}

fn extract_positive_amount(
    processed_tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    account: Address,
    token: Address,
) -> U256 {
    let delta = extract_amount(processed_tx, account, token);
    if delta > I256::ZERO {
        delta.unsigned_abs()
    } else {
        U256::ZERO
    }
}

fn extract_amount(
    processed_tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    account: Address,
    token: Address,
) -> I256 {
    use tx_processor::tx_processor::address_balance_change_calculator::get_token_symbol;

    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&account) {
        if let Some(symbol) = get_token_symbol(&token) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                return amount;
            }
        }
        let token_key = to_checksum_address(&token);
        if let Some(&amount) = balance_changes.token_net.get(&token_key) {
            return amount;
        }
    }
    I256::ZERO
}
