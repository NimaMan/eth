use alloy_primitives::{Address, Bytes, I256, U256};
use eyre::Result;
use reth_chain_query::common_addresses::uniswap_v3_tokens;
use reth_chain_query::to_checksum_address;
use reth_chain_query::tx_builders::uniswap_v4::{
    build_baygus_router_deploy_tx, build_mock_pool_manager_deploy_tx, build_token_approval_tx,
    build_weth_deposit_tx, compute_contract_address, pad_address, pad_u256, CMD_TRANSFER_FROM,
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
    let handle = rt.spawn(async move { run_simulation(simulator_clone).await });

    // Block on the handle to await the result of the spawned task
    let simulation_result = rt.block_on(handle)?;

    // Explicitly drop the original Arc<TxSimulator> reference held in main
    drop(simulator);

    simulation_result
}

async fn run_simulation(simulator: Arc<TxSimulator>) -> Result<()> {
    println!("🦄 Baygus Universal Router - Cross-Protocol Chaining Simulation");
    println!("=============================================================\n");

    let tx_processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("Simulation block: {}", latest_block);

    // 2. Setup Buyer
    let buyer_address: Address = "0x0c96c602b1b332b8ab2093e5d72d804a24bd5689".parse()?;
    println!("Buyer address: {:?}", buyer_address);

    let provider = simulator.provider_factory().provider()?;
    let account = provider.basic_account(&buyer_address)?.unwrap_or_default();
    let deployer_nonce = account.nonce;

    let mut chain = simulator.start_simulation_chain(Some(latest_block)).await?;

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
    println!(
        "[{}] Baygus Router deployed at {}",
        step_index + 1,
        router_address
    );
    step_index += 1;

    // 4. Target:
    // Hop 1: Uniswap V3: WETH -> USDC (Fee 500)
    // Hop 2: SushiSwap: USDC -> WETH

    let tokens_v3 = uniswap_v3_tokens();
    let usdc_info = tokens_v3
        .iter()
        .find(|t| t.symbol == "USDC" && t.fee_tier == 500)
        .unwrap();

    let weth_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
    let usdc_address = usdc_info.token_address;
    let amount_in = U256::from(TEST_AMOUNT_WEI);
    let fee_tier = usdc_info.fee_tier;

    println!("Chaining: WETH -> USDC (UniV3) -> WETH (SushiSwap)");

    // 4.5 Deposit WETH
    let mut deposit_tx = build_weth_deposit_tx(buyer_address, weth_address, amount_in);
    deposit_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut deposit_tx);
    let deposit_result = chain.step_with_trace(deposit_tx.clone()).await?;
    println!(
        "[{}] WETH Deposit: {}",
        step_index + 1,
        if deposit_result.success { "✅" } else { "❌" }
    );
    step_index += 1;

    // 5. Approve Router for WETH
    let mut approve_tx =
        build_token_approval_tx(buyer_address, weth_address, router_address, U256::MAX);
    approve_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut approve_tx);
    chain.step_with_trace(approve_tx.clone()).await?;
    step_index += 1;

    // 6. Execute Chained Swap
    // Command 0: TRANSFER_FROM (User -> Router) WETH amount_in
    // Command 1: V3_SWAP (WETH -> USDC). Recipient = Router (address(this)), spends router balance.
    // Command 2: SUSHISWAP (USDC -> WETH). Recipient = User, spends router balance (amountIn=0 means "use balance").

    // Encoding Command 1 (Uni V3)
    // Input: (ExactInputSingleParams, bool payerIsUser)
    // Params: tokenIn=WETH, tokenOut=USDC, fee=500, recipient=Router, amountIn=1ETH, ...
    let deadline = U256::from(1999999999u64);
    let input_v3 = encode_v3_swap_params(
        weth_address,
        usdc_address,
        fee_tier,
        router_address, // route output to router for the chained hop
        deadline,
        amount_in,
        U256::ZERO,
        U256::ZERO,
    );

    // 3. Sushi Swap (USDC -> WETH) -> User (amountIn=0 uses router balance)
    let path_sushi = vec![usdc_address, weth_address];
    let input_sushi = encode_v2_swap_params(U256::ZERO, U256::ZERO, &path_sushi, buyer_address);

    // Build commands/inputs
    let mut commands_vec = Vec::new();
    let mut inputs_vec = Vec::new();

    // CMD_TRANSFER_FROM: move WETH from user to router
    let mut transfer_input = Vec::new();
    transfer_input.extend_from_slice(&pad_address(weth_address));
    transfer_input.extend_from_slice(&pad_u256(amount_in));
    commands_vec.push(CMD_TRANSFER_FROM);
    inputs_vec.push(Bytes::from(transfer_input));

    // CMD_V3_SWAP = 0x03
    commands_vec.push(0x03);
    inputs_vec.push(Bytes::from(input_v3));

    // CMD_SUSHISWAP = 0x04
    commands_vec.push(0x04);
    inputs_vec.push(Bytes::from(input_sushi));

    let commands = Bytes::from(commands_vec);
    let inputs = inputs_vec;
    let execute_calldata = encode_execute(commands, inputs);

    // Build Transaction
    let mut tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(router_address),
        gas: Some(8_000_000), // Higher gas for multi-hop
        value: Some(U256::ZERO),
        data: Some(execute_calldata),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut tx);

    // Execute
    let result = chain.step_with_trace(tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&tx, &result, latest_block, step_index)
        .await?;

    println!(
        "[{}] Router Execute (Chain V3->Sushi): {} (hash {:#x})",
        step_index + 1,
        if result.success {
            "✅ success"
        } else {
            "❌ failed"
        },
        processed.hash
    );

    if !result.success {
        if let Some(reason) = result.revert_reason.as_deref() {
            println!("⚠️ Revert reason: {reason}");
        }
        return Ok(());
    }

    // Check results
    // We started with 1 WETH. We should get WETH back.
    let weth_received = extract_incoming_amount(&processed, buyer_address, weth_address);
    println!("WETH Returned: {} WETH", format_wei(weth_received));

    debug_log_balance_change(
        "Buyer after Chain",
        &processed,
        buyer_address,
        weth_address,
        usdc_address,
    );

    Ok(())
}

fn encode_v3_swap_params(
    token_in: Address,
    token_out: Address,
    fee: u32,
    recipient: Address,
    deadline: U256,
    amount_in: U256,
    amount_out_min: U256,
    sqrt_price_limit_x96: U256,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_in.as_slice());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_out.as_slice());
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&fee.to_be_bytes());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(recipient.as_slice());
    data.extend_from_slice(&deadline.to_be_bytes::<32>());
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    data.extend_from_slice(&sqrt_price_limit_x96.to_be_bytes::<32>());
    data
}

fn encode_v2_swap_params(
    amount_in: U256,
    amount_out_min: U256,
    path: &[Address],
    recipient: Address,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    // path offset = 5 words * 32 bytes = 160
    data.extend_from_slice(&U256::from(160).to_be_bytes::<32>());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(recipient.as_slice());
    // deadline (uint256)
    data.extend_from_slice(&U256::from(1999999999u64).to_be_bytes::<32>());

    // path array
    data.extend_from_slice(&U256::from(path.len()).to_be_bytes::<32>());
    for addr in path {
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(addr.as_slice());
    }
    data
}

fn apply_simple_gas_policy(tx: &mut UnsignedTransaction) {
    if tx.gas.is_none() {
        tx.gas = Some(5_000_000);
    }
    if tx.max_fee_per_gas.is_none() {
        tx.max_fee_per_gas = Some(50_000_000_000);
    }
    if tx.max_priority_fee_per_gas.is_none() {
        tx.max_priority_fee_per_gas = Some(1_000_000_000);
    }
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
    for body in bodies {
        data.extend_from_slice(&body);
    }
    Bytes::from(data)
}

fn format_wei(wei: U256) -> String {
    format_amount(wei, 18)
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
        let (whole, frac) = padded.split_at(padded.len() - decimals);
        format!("{}.{}", whole, frac.trim_end_matches('0'))
    } else {
        let (whole, frac) = digits.split_at(digits.len() - decimals);
        format!("{}.{}", whole, frac.trim_end_matches('0'))
    }
}

fn extract_incoming_amount(
    processed_tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    account: Address,
    token: Address,
) -> U256 {
    use tx_processor::tx_processor::address_balance_change_calculator::get_token_symbol;
    if let Some(changes) = processed_tx.address_balance_changes.get(&account) {
        // Check WETH (ERC20) movements
        let key = to_checksum_address(&token);
        if let Some(movement) = changes.movements.tokens.get(&key) {
            let total_in: U256 = movement
                .incoming
                .values()
                .fold(U256::ZERO, |acc, v| acc + *v);
            if total_in > U256::ZERO {
                return total_in;
            }
        }

        // Also check if it's tracked as currency (e.g. ETH)
        if let Some(symbol) = get_token_symbol(&token) {
            if let Some(movement) = changes.movements.currencies.get(symbol) {
                // Currency movements might be U256 or I256?
                // Usually movements are absolute amounts (U256).
                let total_in: U256 = movement
                    .incoming
                    .values()
                    .fold(U256::ZERO, |acc, v| acc + *v);
                if total_in > U256::ZERO {
                    return total_in;
                }
            }
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
            let sign = if amount > I256::ZERO {
                "+"
            } else if amount < I256::ZERO {
                "-"
            } else {
                ""
            };
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
