use alloy_primitives::{Address, Bytes, U256};
/// ERC20 Approval Mechanics Demo
///
/// This example demonstrates how ERC20 token approval mechanics work in sequential transactions.
/// It shows that tokens must be approved before they can be transferred by a third party (like a DEX router).
///
/// Real-world scenario: DeFi interactions often require multiple steps in sequence:
/// 1. Approve token spending
/// 2. Execute swap
/// Our sequential simulation can test if these interactions work correctly in order.
use eyre::Result;
use std::str::FromStr;
use tx_simulator::{
    SequentialSimulationOptions, SequentialSimulationResult, TxSimulator, UnsignedTransaction,
};

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Well-known contract addresses that exist and have liquidity
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC token
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"; // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router
const WHALE_ADDRESS: &str = "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5"; // Address with funds
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689"; // Test buyer address

// We'll use latest block dynamically

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 ERC20 Approval Mechanics Demo");
    println!("=================================");
    println!();

    // Initialize simulator
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    println!("✅ Simulator initialized");

    let latest_block = simulator.get_latest_block()?;
    println!("📊 Latest block: {}", latest_block);
    println!();

    // Scenario 1: Direct Swap Without Approval (Should FAIL)
    println!("📋 Scenario 1: Direct Swap Without Approval (Should FAIL)");
    println!("--------------------------------------------------------");
    println!("This tests the restriction: swaps fail when tokens are not approved");

    let scenario1_transactions = vec![create_swap_usdc_for_eth_transaction()];

    let options = SequentialSimulationOptions {
        at_block: Some(latest_block),
        block_header: None,
        stop_on_failure: false, // Continue to see both results
        auto_increment_nonces: true,
        gas_limit_per_tx: Some(300000),
    };

    match simulator
        .simulate_unsigned_tx_sequence(scenario1_transactions, options.clone())
        .await
    {
        Ok(result) => {
            print_sequence_results("Scenario 1", &result);

            // Verify our expectation: transaction should fail
            if result.results.len() >= 1 && !result.results[0].success {
                println!(
                    "✅ EXPECTED BEHAVIOR: Swap failed because USDC is not approved for spending"
                );
            } else {
                println!("❌ UNEXPECTED: Swap should have failed without approval");
            }
        }
        Err(e) => println!("❌ Scenario 1 error: {}", e),
    }
    println!();

    // Scenario 2: Approve → Swap (Should SUCCESS)
    println!("📋 Scenario 2: Approve → Swap (Should SUCCESS)");
    println!("-----------------------------------------------");
    println!("This demonstrates the correct sequence: approval + swap = success");

    let scenario2_transactions = vec![
        create_approve_usdc_transaction(),
        create_swap_usdc_for_eth_transaction(),
    ];

    match simulator
        .simulate_unsigned_tx_sequence(scenario2_transactions, options.clone())
        .await
    {
        Ok(result) => {
            print_sequence_results("Scenario 2", &result);

            // Verify our expectation: all transactions should succeed
            if result.sequence_success {
                println!(
                    "✅ EXPECTED BEHAVIOR: Both transactions succeeded - approval enables swap"
                );
            } else {
                println!("❌ UNEXPECTED: Sequence should have succeeded with proper approval");
            }
        }
        Err(e) => println!("❌ Scenario 2 error: {}", e),
    }
    println!();

    // Scenario 3: ETH → USDC Swap (Should SUCCESS, no approval needed for ETH)
    println!("📋 Scenario 3: ETH → USDC Swap (Should SUCCESS)");
    println!("-----------------------------------------------");
    println!("This tests ETH swaps which don't require approval");

    let scenario3_transactions = vec![create_swap_eth_for_usdc_transaction()];

    match simulator
        .simulate_unsigned_tx_sequence(scenario3_transactions, options)
        .await
    {
        Ok(result) => {
            print_sequence_results("Scenario 3", &result);

            // Verify our expectation: swap should succeed
            if result.sequence_success {
                println!("✅ EXPECTED BEHAVIOR: ETH swap succeeded without approval");
            } else {
                println!("❌ UNEXPECTED: ETH swap should have succeeded");
            }
        }
        Err(e) => println!("❌ Scenario 3 error: {}", e),
    }
    println!();

    println!("💡 Key Takeaways:");
    println!("  - ERC20 tokens require approval before third-party transfers");
    println!("  - Sequential simulation correctly models approval state dependencies");
    println!("  - ETH swaps don't need approval (ETH is not an ERC20 token)");
    println!("  - The approval → swap pattern is fundamental to DeFi interactions");

    Ok(())
}

/// Create USDC approval transaction
/// Approves Uniswap V2 Router to spend USDC
fn create_approve_usdc_transaction() -> UnsignedTransaction {
    // Function: approve(address,uint256)
    let function_signature = "0x095ea7b3"; // approve selector
    let spender = format!("{:0>64}", &UNISWAP_V2_ROUTER[2..]); // Uniswap V2 Router
    let amount = format!("{:0>64x}", 1000_000000u128); // 1000 USDC (6 decimals)

    let data = format!("{}{}{}", function_signature, spender, amount);

    UnsignedTransaction {
        from: Some(Address::from_str(WHALE_ADDRESS).unwrap()),
        to: Some(Address::from_str(USDC_ADDRESS).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(hex::decode(&data[2..]).unwrap())),
        gas: Some(100000),
        gas_price: Some(10_000_000_000), // 10 gwei
        nonce: None,                     // Auto-detected
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create swap USDC for ETH transaction
/// Swaps USDC tokens for ETH via Uniswap V2
fn create_swap_usdc_for_eth_transaction() -> UnsignedTransaction {
    // Function: swapExactTokensForETH(uint256,uint256,address[],address,uint256)
    let function_signature = "0x18cbafe5"; // swapExactTokensForETH selector
    let amount_in = format!("{:0>64x}", 100_000000u128); // 100 USDC (6 decimals)
    let amount_out_min = format!("{:0>64x}", 0); // Accept any amount of ETH
    let path_offset = format!("{:0>64x}", 0xa0); // Offset to path array
    let to_address = format!("{:0>64}", &WHALE_ADDRESS[2..]); // Recipient
    let deadline = format!("{:0>64x}", 1900000000u64); // Far future deadline

    // Path array: [USDC, WETH]
    let path_length = format!("{:0>64x}", 2);
    let usdc_address = format!("{:0>64}", &USDC_ADDRESS[2..]); // USDC
    let weth_address = format!("{:0>64}", &WETH_ADDRESS[2..]); // WETH

    let data = format!(
        "{}{}{}{}{}{}{}{}{}",
        function_signature,
        amount_in,
        amount_out_min,
        path_offset,
        to_address,
        deadline,
        path_length,
        usdc_address,
        weth_address
    );

    UnsignedTransaction {
        from: Some(Address::from_str(WHALE_ADDRESS).unwrap()),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(hex::decode(&data[2..]).unwrap())),
        gas: Some(200000),
        gas_price: Some(10_000_000_000), // 10 gwei
        nonce: None,                     // Auto-detected
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create swap ETH for USDC transaction
/// Swaps ETH for USDC tokens via Uniswap V2
fn create_swap_eth_for_usdc_transaction() -> UnsignedTransaction {
    // Function: swapExactETHForTokens(uint256,address[],address,uint256)
    let function_signature = "0x7ff36ab5"; // swapExactETHForTokens selector
    let amount_out_min = format!("{:0>64x}", 0); // Accept any amount of USDC
    let path_offset = format!("{:0>64x}", 0x80); // Offset to path array
    let to_address = format!("{:0>64}", &TEST_BUYER[2..]); // Recipient
    let deadline = format!("{:0>64x}", 1900000000u64); // Far future deadline

    // Path array: [WETH, USDC]
    let path_length = format!("{:0>64x}", 2);
    let weth_address = format!("{:0>64}", &WETH_ADDRESS[2..]); // WETH
    let usdc_address = format!("{:0>64}", &USDC_ADDRESS[2..]); // USDC

    let data = format!(
        "{}{}{}{}{}{}{}{}",
        function_signature,
        amount_out_min,
        path_offset,
        to_address,
        deadline,
        path_length,
        weth_address,
        usdc_address
    );

    UnsignedTransaction {
        from: Some(Address::from_str(TEST_BUYER).unwrap()),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(U256::from(10_000_000_000_000_000u128)), // 0.01 ETH
        data: Some(Bytes::from(hex::decode(&data[2..]).unwrap())),
        gas: Some(200000),
        gas_price: Some(10_000_000_000), // 10 gwei
        nonce: None,                     // Auto-detected
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Print detailed results of a transaction sequence
fn print_sequence_results(scenario_name: &str, result: &SequentialSimulationResult) {
    println!("🔄 Simulating {} sequence...", scenario_name);
    println!();

    println!("📊 {} Results:", scenario_name);
    println!("  Total transactions: {}", result.total_transactions);
    println!("  Successful: {}", result.successful_transactions);
    println!("  Failed: {}", result.failed_transactions);
    println!("  Total gas used: {}", result.total_gas_used);
    println!("  Sequence success: {}", result.sequence_success);
    println!();

    for (i, tx_result) in result.results.iter().enumerate() {
        let status = if tx_result.success {
            "✅ SUCCESS"
        } else {
            "❌ FAILED"
        };
        let tx_name = match i {
            0 => {
                if result.total_transactions == 1 {
                    "SWAP"
                } else {
                    "APPROVE"
                }
            }
            1 => "SWAP",
            2 => "ADDITIONAL",
            _ => "TRANSACTION",
        };

        println!("  {} Transaction: {}", tx_name, status);
        println!("    Gas used: {}", tx_result.gas_used);

        if let Some(ref reason) = tx_result.revert_reason {
            println!("    Revert reason: {}", reason);
        }
        println!();
    }
}

/// Format address for display (shortened)
fn format_address(address: Address) -> String {
    let addr_str = format!("{:?}", address);
    if addr_str.len() > 10 {
        format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len() - 4..])
    } else {
        addr_str
    }
}
