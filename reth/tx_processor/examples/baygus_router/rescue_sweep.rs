use alloy_primitives::{Address, Bytes, I256, U256};
use eyre::Result;
use reth_chain_query::common_addresses::{uniswap_v2_tokens};
use reth_chain_query::to_checksum_address;
use reth_chain_query::tx_builders::uniswap_v4::{
    build_baygus_router_deploy_tx, build_mock_pool_manager_deploy_tx, build_token_approval_tx,
    build_weth_deposit_tx, compute_contract_address, pad_address, pad_u256,
};
use reth_provider::AccountReader;
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
    println!("🦄 Baygus Universal Router - Rescue/Sweep Simulation");
    println!("==================================================\n");

    let tx_processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("Simulation block: {}", latest_block);

    // 2. Setup Buyer
    let buyer_address: Address = "0x0c96c602b1b332b8ab2093e5d72d804a24bd5689".parse()?;
    println!("Buyer address: {:?}", buyer_address);

    let provider = simulator.provider_factory().provider()?;
    let account = provider.basic_account(&buyer_address)?.unwrap_or_default();
    let deployer_nonce = account.nonce;

    let mut chain = simulator
        .start_simulation_chain(Some(latest_block))
        .await?;

    let mut step_index = 0u64;

    // 3. Deploy Mock Ecosystem
    let mut mock_pm_deploy_tx = build_mock_pool_manager_deploy_tx(buyer_address);
    mock_pm_deploy_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut mock_pm_deploy_tx);
    chain.step_with_trace(mock_pm_deploy_tx.clone()).await?;
    let mock_pm_address = compute_contract_address(buyer_address, deployer_nonce + step_index);
    step_index += 1;

    // Deploy Baygus Router
    let mut deploy_tx = build_baygus_router_deploy_tx(buyer_address, mock_pm_address);
    deploy_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut deploy_tx);
    chain.step_with_trace(deploy_tx.clone()).await?;
    let router_address = compute_contract_address(buyer_address, deployer_nonce + step_index);
    println!("[{}] Baygus Router deployed at {}", step_index + 1, router_address);
    step_index += 1;

    // 4. Scenario: Accidental Token Transfer to Router
    // Transfer USDC to Router without calling execute (simulating stuck funds)
    // First get USDC via V2
    let tokens = uniswap_v2_tokens();
    let usdc_info = tokens.iter().find(|t| t.symbol == "USDC").unwrap();
    let weth_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
    let usdc_address = usdc_info.token_address;
    let amount_in = U256::from(TEST_AMOUNT_WEI);

    // 4.5 Deposit WETH
    let mut deposit_tx = build_weth_deposit_tx(buyer_address, weth_address, amount_in);
    deposit_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut deposit_tx);
    chain.step_with_trace(deposit_tx.clone()).await?;
    step_index += 1;

    // 5. Swap WETH -> USDC (V2) via Router (to get USDC)
    let mut approve_tx = build_token_approval_tx(buyer_address, weth_address, router_address, U256::MAX);
    approve_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut approve_tx);
    chain.step_with_trace(approve_tx.clone()).await?;
    step_index += 1;

    let path_buy = vec![weth_address, usdc_address];
    let input_buy = encode_v2_swap_params(amount_in, U256::ZERO, &path_buy, buyer_address, true);
    // Prepend CMD_TRANSFER_FROM to fund the router with WETH before swapping.
    let mut commands_buy_vec = Vec::new();
    let mut inputs_buy_vec = Vec::new();

    // CMD_TRANSFER_FROM (0x0a)
    let mut transfer_input = Vec::new();
    transfer_input.extend_from_slice(&pad_address(weth_address));
    transfer_input.extend_from_slice(&pad_u256(amount_in));
    commands_buy_vec.push(0x0a);
    inputs_buy_vec.push(Bytes::from(transfer_input));

    // CMD_V2_SWAP (0x02)
    commands_buy_vec.push(0x02);
    inputs_buy_vec.push(Bytes::from(input_buy));

    let commands_buy = Bytes::from(commands_buy_vec);
    let inputs_buy = inputs_buy_vec;
    let execute_calldata_buy = encode_execute(commands_buy, inputs_buy);

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
    let buy_res = chain.step_with_trace(buy_tx.clone()).await?;
    println!("[{}] Buy V2: {}", step_index, if buy_res.success { "✅" } else { "❌" });
    if !buy_res.success {
        if let Some(reason) = buy_res.revert_reason.as_deref() {
            println!("⚠️ Buy Revert: {reason}");
        }
        return Ok(());
    }

    let buy_processed = tx_processor.process_transaction_from_simulation_result(&buy_tx, &buy_res, latest_block, step_index).await?;
    let usdc_balance = extract_positive_amount(&buy_processed, buyer_address, usdc_address);
    println!("Buyer has {} USDC", format_amount(usdc_balance, 6));
    step_index += 1;

    if usdc_balance.is_zero() {
        println!("⚠️ Buyer has 0 USDC, cannot test accidental transfer.");
        return Ok(());
    }

    // 6. Accidental Transfer: User sends USDC to Router directly (ERC20 transfer)
    // We construct a raw ERC20 transfer transaction
    let transfer_calldata = Bytes::from([
        &hex::decode("a9059cbb").unwrap()[..], // transfer(address,uint256)
        &[0u8; 12], &router_address.as_slice(),
        &usdc_balance.to_be_bytes::<32>(),
    ].concat());

    let mut accidental_tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(usdc_address), // Call Token Contract
        gas: Some(100_000),
        value: Some(U256::ZERO),
        data: Some(transfer_calldata),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut accidental_tx);
    
    let acc_res = chain.step_with_trace(accidental_tx.clone()).await?;
    println!("[{}] Accidental Transfer to Router: {}", step_index + 1, if acc_res.success { "✅" } else { "❌" });
    step_index += 1;

    // Verify Router has USDC
    // We can't easily query state in this sim harness without a view call, but we can try to sweep.

    // 7. Rescue / Sweep
    // Command: CMD_SWEEP (0x07)
    // Input: (address token, address recipient, uint256 amountMinimum)
    let input_sweep = encode_sweep_params(usdc_address, buyer_address, U256::from(1));
    let commands_sweep = Bytes::from(vec![0x07]);
    let inputs_sweep = vec![Bytes::from(input_sweep)];
    let execute_calldata_sweep = encode_execute(commands_sweep, inputs_sweep);

    let mut sweep_tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(router_address),
        gas: Some(200_000),
        value: Some(U256::ZERO),
        data: Some(execute_calldata_sweep),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut sweep_tx);

    let sweep_res = chain.step_with_trace(sweep_tx.clone()).await?;
    let sweep_processed = tx_processor.process_transaction_from_simulation_result(&sweep_tx, &sweep_res, latest_block, step_index).await?;

    println!(
        "[{}] Router Execute (Sweep): {} (hash {:#x})",
        step_index + 1,
        if sweep_res.success { "✅ success" } else { "❌ failed" },
        sweep_processed.hash
    );

    if !sweep_res.success {
        if let Some(reason) = sweep_res.revert_reason.as_deref() {
            println!("⚠️ Revert reason: {reason}");
        }
        return Ok(())
    }

    let recovered = extract_positive_amount(&sweep_processed, buyer_address, usdc_address);
    println!("Recovered: {} USDC", format_amount(recovered, 6));
    
    debug_log_balance_change("Buyer after Sweep", &sweep_processed, buyer_address, weth_address, usdc_address);

    Ok(())
}

// Helper encoders
fn encode_v2_swap_params(
    amount_in: U256,
    amount_out_min: U256,
    path: &[Address],
    recipient: Address,
    payer_is_user: bool
) -> Vec<u8> {
    let mut data = Vec::new();
    // Head
    data.extend_from_slice(&amount_in.to_be_bytes::<32>()); // 0
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>()); // 32
    
    let path_offset = U256::from(160); // 5 * 32 = 160 (amountIn, min, offset, recipient, bool)
    data.extend_from_slice(&path_offset.to_be_bytes::<32>()); // 64
    
    data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(recipient.as_slice()); // 96
    
    // bool payerIsUser at 128
    data.extend_from_slice(&[0u8; 31]);
    data.extend_from_slice(&[if payer_is_user { 1 } else { 0 }]); // 128
    
    // Body (Path) starts at 160
    let path_len = U256::from(path.len());
    data.extend_from_slice(&path_len.to_be_bytes::<32>());
    for addr in path {
        data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(addr.as_slice());
    }
    data
}

fn encode_sweep_params(token: Address, recipient: Address, amount_min: U256) -> Vec<u8> {
    // (address token, address recipient, uint256 amountMinimum)
    let mut data = Vec::new();
    data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(token.as_slice());
    data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(recipient.as_slice());
    data.extend_from_slice(&amount_min.to_be_bytes::<32>());
    data
}

fn encode_execute(commands: Bytes, inputs: Vec<Bytes>) -> Bytes {
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
    for body in bodies { data.extend_from_slice(&body); }
    Bytes::from(data)
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
            let sign = if amount > I256::ZERO { "+" } else if amount < I256::ZERO { "-" } else { "" };
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
