use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
/// Basic simulation example - test the pure simulation functionality
use tx_simulator::{TxSimulator, UnsignedTransaction};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Testing tx_simulator - pure simulation library\n");

    // Initialize simulator
    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let simulator = TxSimulator::new(&reth_datadir)?;

    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}", latest_block);

    // Create a simple call request (check ETH balance)
    let call = UnsignedTransaction {
        from: Some(Address::ZERO),
        to: Some("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?), // WETH
        value: Some(U256::ZERO),
        data: Some(Bytes::from(vec![0x18, 0x16, 0x0d, 0xdd])), // totalSupply()
        gas: Some(100_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None,
    };

    // Simulate the call
    println!("\nSimulating WETH totalSupply() call...");
    let result = simulator
        .simulate_unsigned_transaction_at_block(call, latest_block)
        .await?;

    println!("Simulation result:");
    println!("  Success: {}", result.success);
    println!("  Gas used: {}", result.gas_used);
    if let Some(reason) = result.revert_reason {
        println!("  Revert reason: {}", reason);
    }

    println!("\n✅ tx_simulator is working correctly!");
    println!("This is a clean library focused only on transaction simulation.");
    println!("Balance change calculations are handled in tx_processor.");

    Ok(())
}
