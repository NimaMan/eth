use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use reth_chain_query::tx_builders::uniswap_v4::{
    build_baygus_router_deploy_tx, build_mock_pool_manager_deploy_tx, build_weth_deposit_tx,
    compute_contract_address,
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
    println!("🦄 Baygus Universal Router - Balancer Flash Loan Simulation");
    println!("========================================================\n");

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

    let weth_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
    let loan_amount = U256::from(TEST_AMOUNT_WEI); // 1 ETH Loan

    // 4. Fund Router for Fees (0.1 ETH)
    let dust_amount = U256::from(100_000_000_000_000_000u128);
    let mut deposit_tx = build_weth_deposit_tx(buyer_address, weth_address, dust_amount);
    deposit_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut deposit_tx);
    chain.step_with_trace(deposit_tx.clone()).await?;
    step_index += 1;

    let transfer_calldata = Bytes::from(
        [
            &hex::decode("a9059cbb").unwrap()[..],
            &[0u8; 12],
            &router_address.as_slice(),
            &dust_amount.to_be_bytes::<32>(),
        ]
        .concat(),
    );
    let mut fund_tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(weth_address),
        gas: Some(100_000),
        value: Some(U256::ZERO),
        data: Some(transfer_calldata),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut fund_tx);
    chain.step_with_trace(fund_tx.clone()).await?;
    step_index += 1;

    println!("Router funded with 0.1 WETH for fees.");

    // 5. Execute Flash Loan
    // Input: (address[] tokens, uint256[] amounts, bytes userData)
    // userData = encoded (bytes commands, bytes[] inputs)
    let inner_commands = Bytes::new();
    let inner_inputs: Vec<Bytes> = vec![];
    let user_data = encode_abi_bytes_bytes_array(inner_commands, inner_inputs);

    let tokens = vec![weth_address];
    let amounts = vec![loan_amount];
    let input_flash = encode_flash_loan_params(tokens, amounts, user_data);

    let commands = Bytes::from(vec![0x08]); // CMD_BALANCER_FLASH_LOAN
    let inputs = vec![Bytes::from(input_flash)];
    let execute_calldata = encode_execute(commands, inputs);

    let mut tx = UnsignedTransaction {
        from: Some(buyer_address),
        to: Some(router_address),
        gas: Some(1_000_000),
        value: Some(U256::ZERO),
        data: Some(execute_calldata),
        nonce: Some(deployer_nonce + step_index),
        ..Default::default()
    };
    apply_simple_gas_policy(&mut tx);

    let result = chain.step_with_trace(tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&tx, &result, latest_block, step_index)
        .await?;

    println!(
        "[{}].Router Execute (Flash Loan): {} (hash {:#x})",
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

    println!("Logs count: {}", result.logs.len());
    Ok(())
}

fn encode_abi_bytes_bytes_array(_a: Bytes, _b: Vec<Bytes>) -> Bytes {
    // Encodes empty (bytes, bytes[])
    let mut data = Vec::new();
    data.extend_from_slice(&U256::from(64).to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(96).to_be_bytes::<32>());
    data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>()); // len a
    data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>()); // len b
    Bytes::from(data)
}

fn encode_flash_loan_params(tokens: Vec<Address>, amounts: Vec<U256>, user_data: Bytes) -> Vec<u8> {
    let mut data = Vec::new();
    let off_tokens = 96;
    let len_tokens = tokens.len() * 32 + 32;
    let off_amounts = off_tokens + len_tokens;
    let len_amounts = amounts.len() * 32 + 32;
    let off_userdata = off_amounts + len_amounts;

    data.extend_from_slice(&U256::from(off_tokens).to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(off_amounts).to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(off_userdata).to_be_bytes::<32>());

    data.extend_from_slice(&U256::from(tokens.len()).to_be_bytes::<32>());
    for t in tokens {
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(t.as_slice());
    }

    data.extend_from_slice(&U256::from(amounts.len()).to_be_bytes::<32>());
    for a in amounts {
        data.extend_from_slice(&a.to_be_bytes::<32>());
    }

    data.extend_from_slice(&U256::from(user_data.len()).to_be_bytes::<32>());
    data.extend_from_slice(&user_data);
    let padding = (user_data.len() + 31) / 32 * 32 - user_data.len();
    data.extend_from_slice(&vec![0u8; padding]);

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
    for body in bodies {
        data.extend_from_slice(&body);
    }
    Bytes::from(data)
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
