use alloy_primitives::{Address, Bytes, I256, U256};
use eyre::Result;
use reth_chain_query::common_addresses::{uniswap_v2_tokens, uniswap_v3_tokens, sushiswap_tokens};
use reth_chain_query::to_checksum_address;
use reth_chain_query::tx_builders::uniswap_v4::{
    build_baygus_router_deploy_tx,
    build_mock_pool_manager_deploy_tx,
    build_token_approval_tx,
    build_weth_deposit_tx,
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

    // 4. Target:
    // Hop 1: Uniswap V3: WETH -> USDC (Fee 500)
    // Hop 2: SushiSwap: USDC -> WETH
    
    let tokens_v3 = uniswap_v3_tokens();
    let usdc_info = tokens_v3.iter().find(|t| t.symbol == "USDC" && t.fee_tier == 500).unwrap();
    
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
    println!("[{}] WETH Deposit: {}", step_index + 1, if deposit_result.success { "✅" } else { "❌" });
    step_index += 1;

    // 5. Approve Router for WETH
    let mut approve_tx = build_token_approval_tx(buyer_address, weth_address, router_address, U256::MAX);
    approve_tx.nonce = Some(deployer_nonce + step_index);
    apply_simple_gas_policy(&mut approve_tx);
    chain.step_with_trace(approve_tx.clone()).await?;
    step_index += 1;

    // 6. Execute Chained Swap
    // Command 1: V3_SWAP (WETH -> USDC). Recipient = Router (address(this)). Payer = User (true).
    // Command 2: SUSHISWAP (USDC -> WETH). Recipient = User. Payer = Router (false).

    // Encoding Command 1 (Uni V3)
    // Input: (ExactInputSingleParams, bool payerIsUser)
    // Params: tokenIn=WETH, tokenOut=USDC, fee=500, recipient=Router, amountIn=1ETH, ...
    let deadline = U256::from(1999999999u64);
    let params_v3 = encode_v3_swap_params(
        weth_address,
        usdc_address,
        fee_tier,
        router_address, // Recipient is Router!
        deadline,
        amount_in,
        U256::ZERO,
        U256::ZERO
    );
    let input_v3 = encode_with_bool(params_v3, true); // payerIsUser = true

    // Encoding Command 2 (Sushi)
    // Input: (amountIn=0???, amountOutMin=0, path=[USDC, WETH], recipient=User, bool payerIsUser)
    // ISSUE: We don't know the exact amount of USDC received from V3 yet because it's atomic.
    // The current router implementation requires explicit `amountIn`.
    // To support chaining dynamically, the router needs to support "use balance" or we must predict amount.
    // For this simulation, since we decode `amountIn` in `_v2Swap`, if we pass 0, it will try to approve 0.
    // BUT `_v2Swap` does `approve(router, amountIn)`.
    // If we want to use the *entire* balance, we need a way to signal "use balance of tokenIn".
    // Common pattern: `amountIn = type(uint256).max` means "balanceOf(this)".
    // Or we modify `_v2Swap` to handle a magic value or simply check balance if `!payerIsUser`.
    
    // Let's update the Router logic to handle `amountIn == 0` (or max) when `!payerIsUser` as "use full balance".
    // For now, let's simulate "User -> Router -> User" in two separate txs to verify logic first?
    // No, the goal is chaining.
    // I will assume for now I have to predict the amount OR update router.
    // Let's update `BaygusRouter` `_v2Swap` (and others) to read balance if `amountIn == 0` and `!payerIsUser`.
    
    // Wait, I can't update Router mid-simulation script writing.
    // I should have updated Router logic for dynamic amounts.
    // But let's see if I can cheat by predicting the output?
    // No, that's hard.
    // I will proceed to update `BaygusRouter.sol` first to support dynamic amounts before finishing this script. 
    
    return Ok(());
}

fn encode_with_bool(mut data: Vec<u8>, flag: bool) -> Vec<u8> {
    // bool is encoded as uint256 (32 bytes) 0 or 1
    data.extend_from_slice(&[0u8; 31]);
    data.extend_from_slice(&[if flag { 1 } else { 0 }]);
    data
}

// ... (Copy existing encoders and helpers) ...
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
    data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(token_in.as_slice());
    data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(token_out.as_slice());
    data.extend_from_slice(&[0u8; 28]); data.extend_from_slice(&fee.to_be_bytes());
    data.extend_from_slice(&[0u8; 12]); data.extend_from_slice(recipient.as_slice());
    data.extend_from_slice(&deadline.to_be_bytes::<32>());
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    data.extend_from_slice(&sqrt_price_limit_x96.to_be_bytes::<32>());
    data
}

fn apply_simple_gas_policy(tx: &mut UnsignedTransaction) {
    if tx.gas.is_none() { tx.gas = Some(5_000_000); }
    if tx.max_fee_per_gas.is_none() { tx.max_fee_per_gas = Some(50_000_000_000); }
    if tx.max_priority_fee_per_gas.is_none() { tx.max_priority_fee_per_gas = Some(1_000_000_000); }
}
