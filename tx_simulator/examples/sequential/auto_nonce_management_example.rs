use alloy_primitives::{Address, U256};
use eyre::Result;
use tx_simulator::{SequentialSimulationOptions, TxSimulator, UnsignedTransaction};

/// Auto nonce detection and sequence increment example.
///
/// The simulator does not rewrite explicitly wrong nonces. The safe default is
/// to omit `nonce` so the simulator reads it from the selected state, then lets
/// stateful sequence APIs advance the tracked nonce after each executed tx.
#[tokio::main]
async fn main() -> Result<()> {
    println!("Auto Nonce Management Example");
    println!("=============================");

    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let simulator = TxSimulator::new(&reth_datadir)?;

    let from_address = "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5".parse::<Address>()?;
    let to_address = "0x388C818CA8B9251b393131C08a736A67ccB19297".parse::<Address>()?;

    let mut request = UnsignedTransaction {
        from: Some(from_address),
        to: Some(to_address),
        value: Some(U256::from(1u64)),
        gas: Some(21_000),
        gas_price: Some(20_000_000_000),
        nonce: Some(0),
        ..Default::default()
    };

    println!();
    println!("1. Explicit wrong nonce");
    match simulator
        .simulate_unsigned_transaction(request.clone())
        .await
    {
        Ok(result) => {
            println!("   Simulation executed. success={}", result.success);
            if let Some(reason) = result.revert_reason {
                println!("   Revert reason: {}", reason);
            }
        }
        Err(err) => {
            println!("   Expected nonce/state validation error: {}", err);
        }
    }

    println!();
    println!("2. Omitted nonce");
    request.nonce = None;
    match simulator
        .simulate_unsigned_transaction(request.clone())
        .await
    {
        Ok(result) => {
            println!("   Simulation executed with state-derived nonce.");
            println!(
                "   success={}, gas_used={}",
                result.success, result.gas_used
            );
        }
        Err(err) => {
            println!("   Simulation failed: {}", err);
        }
    }

    println!();
    println!("3. Stateful sequence");
    let options = SequentialSimulationOptions {
        stop_on_failure: false,
        auto_increment_nonces: true,
        ..Default::default()
    };
    let sequence = simulator
        .simulate_unsigned_tx_sequence(vec![request.clone(), request], options)
        .await?;

    for tx in &sequence.results {
        println!(
            "   tx {}: success={}, gas_used={}, tracked_nonces={:?}",
            tx.transaction_index, tx.success, tx.gas_used, tx.updated_nonces
        );
    }

    Ok(())
}
