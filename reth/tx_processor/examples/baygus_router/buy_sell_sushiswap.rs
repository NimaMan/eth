use alloy_primitives::{Address, I256, U256};
use eyre::Result;
use reth_chain_query::common_addresses::sushiswap_tokens;
use reth_chain_query::to_checksum_address;
use reth_chain_query::tx_builders::uniswap_v4::{
    build_baygus_router_deploy_tx,
    build_mock_pool_manager_deploy_tx,
    build_token_approval_tx,
    build_weth_deposit_tx,
    compute_contract_address,
    pad_address,
    pad_u256,
    CMD_TRANSFER_FROM,
};
use reth_provider::AccountExtReader;
use std::sync::Arc;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::UnsignedTransaction;
use tx_simulator::TxSimulator;

const TEST_AMOUNT_WEI: u128 = 1_000_000_000_000_000_000; // 1 ETH

fn main() -> Result<()> {
    // 1. Setup Simulator (Synchronous)
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    println!("Reth datadir: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);

    // Create a runtime for the simulation execution
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // Spawn the simulation logic as a separate Tokio task
    let simulator_clone = simulator.clone();
    let handle = rt.spawn(async move {
        run_simulation(simulator_clone).await
    });

    // Block on the handle to await the result of the spawned task
    let simulation_result = rt.block_on(handle)?;

    // Explicitly drop the original Arc<TxSimulator> reference held in main
    drop(simulator);

    simulation_result
}

async fn run_simulation(simulator: Arc<TxSimulator>) -> Result<()> {
    println!("🦄 Baygus Universal Router - SushiSwap Buy/Sell Simulation");
    println!("========================================================\n");

    let tx_processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("Simulation block: {}", latest_block);

    // 2. Setup Buyer
    let buyer_address: Address = "0x0c96c602b1b332b8ab2093e5d72d804a24bd5689".parse()?;
    println!("Buyer address: {:?}", buyer_address);

    let provider = simulator.provider_factory().provider()?;
    let account = provider.basic_accounts(vec![buyer_address])?
        .into_iter()
        .find(|(addr, _)| addr == &buyer_address)
        .map(|(_, acc_opt)| acc_opt)
        .flatten()
        .unwrap_or_default();
    let deployer_nonce = account.nonce;

    let mut chain = simulator
        .start_simulation_chain(Some(latest_block))
        .await?;

    let mut step_index = 0u64;

    // 3. Deploy Mock Ecosystem (Required for BaygusRouter constructor)
    let mut mock_pm_deploy_tx = build_mock_pool_manager_deploy_tx(buyer_address);
    mock_pm_deploy_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut mock_pm_deploy_tx);
    chain.step_with_trace(mock_pm_deploy_tx.clone()).await?;
    let mock_pm_address = compute_contract_address(buyer_address, deployer_nonce + step_index);
    println!("[{}] Mock PoolManager deployed at {}", step_index + 1, mock_pm_address);
    step_index += 1;

    // Deploy Baygus Router
    let mut deploy_tx = build_baygus_router_deploy_tx(buyer_address, mock_pm_address);
    deploy_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut deploy_tx);
    chain.step_with_trace(deploy_tx.clone()).await?;
    let router_address = compute_contract_address(buyer_address, deployer_nonce + step_index);
    println!("[{}] Baygus Router deployed at {}", step_index + 1, router_address);
    step_index += 1;

    // 4. Target Real SushiSwap V2 Pool (USDC/WETH)
    let tokens = sushiswap_tokens();
    let token_info = tokens
        .iter()
        .find(|t| t.symbol == "USDC")
        .ok_or_else(|| eyre::eyre!("USDC not found in sushiswap_tokens"))?;
    
    // SushiSwap WETH address is standard
    let weth_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
    let usdc_address = token_info.token_address;
    let amount_in = U256::from(TEST_AMOUNT_WEI);

    println!("Swapping {} WETH for USDC on SushiSwap", format_wei(amount_in));

    // 4.5 Deposit WETH
    let mut deposit_tx = build_weth_deposit_tx(buyer_address, weth_address, amount_in);
    deposit_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut deposit_tx);
    let deposit_result = chain.step_with_trace(deposit_tx.clone()).await?;
    println!("[{}] WETH Deposit: {}", step_index + 1, if deposit_result.success { "✅" } else { "❌" });
    step_index += 1;

    // 5. Approve Router to spend WETH
    let mut approve_tx = build_token_approval_tx(buyer_address, weth_address, router_address, U256::MAX);
    approve_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut approve_tx);
    let approve_result = chain.step_with_trace(approve_tx.clone()).await?;
    println!("[{}] WETH Approval: {}", step_index + 1, if approve_result.success { "✅" } else { "❌" });
    step_index += 1;

    // 6. Execute Buy (WETH -> USDC)
    let path_buy = vec![weth_address, usdc_address];
    let amount_out_min = U256::ZERO; // Slippage ignored for sim

    use alloy_primitives::Bytes;

    // Commands and Inputs for execute
    let mut commands_buy_vec = Vec::new();
    let mut inputs_buy_vec = Vec::new();

    // 1. Prepend CMD_TRANSFER_FROM for WETH
    let mut transfer_in_input = Vec::new();
    transfer_in_input.extend_from_slice(&pad_address(weth_address));
    transfer_in_input.extend_from_slice(&pad_u256(amount_in));
    commands_buy_vec.push(CMD_TRANSFER_FROM);
    inputs_buy_vec.push(Bytes::from(transfer_in_input));

    // 2. Add SushiSwap command and input
    let input_buy_sushi_swap = encode_v2_swap_params(amount_in, amount_out_min, &path_buy, buyer_address);
    commands_buy_vec.push(0x04); // CMD_SUSHISWAP
    inputs_buy_vec.push(Bytes::from(input_buy_sushi_swap));
    
    let commands_buy = Bytes::from(commands_buy_vec);
    let execute_calldata_buy = encode_execute(commands_buy, inputs_buy_vec);

    let mut buy_tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(router_address),
        gas: Some(5_000_000),
        value: Some(U256::ZERO),
        data: Some(execute_calldata_buy),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut buy_tx);

    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(
            &buy_tx,
            &buy_result,
            latest_block,
            step_index,
        )
        .await?;

    println!(
        "[{}] Router Execute (Buy Sushi): {} (hash {:#x})",
        step_index + 1,
        if buy_result.success { "✅ success" } else { "❌ failed" },
        buy_processed.hash
    );

    if !buy_result.success {
        if let Some(reason) = buy_result.revert_reason.as_deref() {
            println!("⚠️ Revert reason: {reason}");
        }
        if let Some(ctx) = buy_result.revert_context {
            println!(
                "Revert context   : target {} (calldata {} bytes)",
                ctx.target,
                ctx.calldata_len
            );
        }
        return Ok(())
    }
    step_index += 1;

    // Check received USDC amount
    let tokens_received = extract_positive_amount(&buy_processed, buyer_address, usdc_address);
    println!("Tokens Received : {} USDC", format_amount(tokens_received, 6));
    debug_log_balance_change("Buyer after Buy", &buy_processed, buyer_address, weth_address, usdc_address);

    if tokens_received.is_zero() {
        println!("⚠️ No tokens received, skipping sell.");
        return Ok(())
    }

    // 7. Approve Router to spend USDC
    let mut approve_usdc_tx = build_token_approval_tx(buyer_address, usdc_address, router_address, tokens_received);
    approve_usdc_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut approve_usdc_tx);
    let approve_usdc_result = chain.step_with_trace(approve_usdc_tx.clone()).await?;
    println!("[{}] USDC Approval: {}", step_index + 1, if approve_usdc_result.success { "✅" } else { "❌" });
    step_index += 1;

    // 8. Execute Sell (USDC -> WETH)
    let path_sell = vec![usdc_address, weth_address];

    let mut commands_sell_vec = Vec::new();
    let mut inputs_sell_vec = Vec::new();

    // 1. Prepend CMD_TRANSFER_FROM for USDC
    let mut transfer_out_input = Vec::new();
    transfer_out_input.extend_from_slice(&pad_address(usdc_address));
    transfer_out_input.extend_from_slice(&pad_u256(tokens_received));
    commands_sell_vec.push(CMD_TRANSFER_FROM);
    inputs_sell_vec.push(Bytes::from(transfer_out_input));

    // 2. Add SushiSwap command and input
    let input_sell_sushi_swap = encode_v2_swap_params(tokens_received, U256::ZERO, &path_sell, buyer_address);
    commands_sell_vec.push(0x04); // CMD_SUSHISWAP
    inputs_sell_vec.push(Bytes::from(input_sell_sushi_swap));

    let commands_sell = Bytes::from(commands_sell_vec);
    let execute_calldata_sell = encode_execute(commands_sell, inputs_sell_vec);

    let mut sell_tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(router_address),
        gas: Some(5_000_000),
        value: Some(U256::ZERO),
        data: Some(execute_calldata_sell),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut sell_tx);

    let sell_result = chain.step_with_trace(sell_tx.clone()).await?;
    let sell_processed = tx_processor
        .process_transaction_from_simulation_result(
            &sell_tx,
            &sell_result,
            latest_block,
            step_index,
        )
        .await?;

    println!(
        "[{}] Router Execute (Sell Sushi): {} (hash {:#x})",
        step_index + 1,
        if sell_result.success { "✅ success" } else { "❌ failed" },
        sell_processed.hash
    );

    if !sell_result.success {
        if let Some(reason) = sell_result.revert_reason.as_deref() {
            println!("⚠️ Revert reason: {reason}");
        }
        return Ok(())
    }

    let weth_received = extract_positive_amount(&sell_processed, buyer_address, weth_address);
    println!("WETH Received   : {} WETH", format_wei(weth_received));

    debug_log_balance_change("Buyer after Sell", &sell_processed, buyer_address, weth_address, usdc_address);

    Ok(())
}

// Helpers (same as before)
fn encode_v2_swap_params(amount_in: U256, amount_out_min: U256, path: &[Address], recipient: Address) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    
    let path_offset = U256::from(128);
    data.extend_from_slice(&path_offset.to_be_bytes::<32>());
    
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(recipient.as_slice());
    
    let path_len = U256::from(path.len());
    data.extend_from_slice(&path_len.to_be_bytes::<32>());
    for addr in path {
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(addr.as_slice());
    }
    data
}

fn encode_execute(commands: alloy_primitives::Bytes, inputs: Vec<alloy_primitives::Bytes>) -> alloy_primitives::Bytes {
    let selector = [0x24, 0x85, 0x6b, 0xc3];
    let mut data = selector.to_vec();
    
    data.extend_from_slice(&U256::from(64).to_be_bytes::<32>());
    let padded_commands_len = (commands.len() + 31) / 32 * 32;
    let inputs_offset = 64 + 32 + padded_commands_len;
    data.extend_from_slice(&U256::from(inputs_offset).to_be_bytes::<32>());
    
    data.extend_from_slice(&U256::from(commands.len()).to_be_bytes::<32>());
    data.extend_from_slice(&commands);
    let padding = padded_commands_len - commands.len();
    data.extend_from_slice(&vec![0u8; padding]);
    
    data.extend_from_slice(&U256::from(inputs.len()).to_be_bytes::<32>());
    
    let mut body_offset = inputs.len() * 32;
    let mut bodies = Vec::new();
    
    for input in &inputs {
        data.extend_from_slice(&U256::from(body_offset).to_be_bytes::<32>());
        let mut body = Vec::new();
        body.extend_from_slice(&U256::from(input.len()).to_be_bytes::<32>());
        body.extend_from_slice(input);
        let p = (input.len() + 31) / 32 * 32 - input.len();
        body.extend_from_slice(&vec![0u8; p]);
        bodies.push(body);
        body_offset += 32 + input.len() + p;
    }
    
    for body in bodies {
        data.extend_from_slice(&body);
    }
    
    alloy_primitives::Bytes::from(data)
}

fn apply_simple_gas_policy(tx: &mut UnsignedTransaction) {
    if tx.gas.is_none() { tx.gas = Some(5_000_000); }
    if tx.max_fee_per_gas.is_none() { tx.max_fee_per_gas = Some(50_000_000_000); }
    if tx.max_priority_fee_per_gas.is_none() { tx.max_priority_fee_per_gas = Some(1_000_000_000); }
}

fn format_wei(wei: U256) -> String {
    format_amount(wei, 18)
}

fn format_amount(amount: U256, decimals: u8) -> String {
    if amount.is_zero() { return "0".to_string(); }
    let digits = amount.to_string();
    let decimals = decimals as usize;
    if decimals == 0 { return digits; }
    
    if digits.len() <= decimals {
        let padded = format!("{:0>width$}", digits, width = decimals + 1);
        let (whole, frac) = padded.split_at(padded.len() - decimals);
        format!("{}.{}", whole, frac.trim_end_matches('0'))
    } else {
        let (whole, frac) = digits.split_at(digits.len() - decimals);
        format!("{}.{}", whole, frac.trim_end_matches('0'))
    }
}

fn extract_positive_amount(
    processed_tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    account: Address,
    token: Address,
) -> U256 {
    use tx_processor::tx_processor::address_balance_change_calculator::get_token_symbol;
    if let Some(changes) = processed_tx.address_balance_changes.get(&account) {
        if let Some(symbol) = get_token_symbol(&token) {
            if let Some(&amount) = changes.currency_net.get(symbol) {
                if amount > I256::ZERO { return amount.unsigned_abs(); }
            }
        }
        let key = to_checksum_address(&token);
        if let Some(&amount) = changes.token_net.get(&key) {
            if amount > I256::ZERO { return amount.unsigned_abs(); }
        }
    }
    U256::ZERO
}

fn debug_log_balance_change(
    label: &str,
    processed_tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    address: Address,
    weth_address: Address,
    usdc_address: Address,
) {
    use std::collections::HashMap;
    use tx_processor::tx_processor::address_balance_change_calculator::get_token_symbol;

    println!("\n--- {} Balance Changes ---", label);
    if let Some(changes) = processed_tx.address_balance_changes.get(&address) {
        let mut currency_changes: HashMap<String, I256> = HashMap::new();

        for (sym, amount) in &changes.currency_net {
            currency_changes.insert(sym.clone(), *amount);
        }
        for (addr_key, amount) in &changes.token_net {
            if let Ok(addr) = addr_key.parse::<Address>() {
                if addr == weth_address {
                    currency_changes.insert("mWETH".to_string(), *amount);
                } else if addr == usdc_address {
                    currency_changes.insert("mUSDC".to_string(), *amount);
                } else if let Some(sym) = get_token_symbol(&addr) {
                    currency_changes.insert(sym.to_string(), *amount);
                } else {
                    currency_changes.insert(format!("Token({:?})", addr), *amount);
                }
            }
        }

        for (currency, amount) in currency_changes {
            let sign = if amount > I256::ZERO { "+" } else { "" };
            let display_amount = if currency == "ETH" || currency == "WETH" || currency == "mWETH" {
                format_wei(amount.unsigned_abs())
            } else {
                format_amount(amount.unsigned_abs(), 6)
            };
            println!("  {}: {}{}", currency, sign, display_amount);
        }
    } else {
        println!("  No changes recorded for {}", address);
    }
    println!("--------------------------");
}
