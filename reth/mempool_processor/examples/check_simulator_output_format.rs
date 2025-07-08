/// Check what format our simulator actually returns for state changes

use mempool_processor::tx_simulator::DirectTxSimulator;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::H256;
use eyre::Result;
use std::str::FromStr;
use std::fs::File;
use std::io::Write;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    // Use a recent transaction that we know affected pools
    // Let's use one from our logs that shows "POOL AFFECTED"
    let test_tx = "0xe3dfce8651161da7e4656c2cb348ae403219e95b29f3d55472dca7c5ca7abbb0"; // From earlier logs
    
    info!("Testing simulator output format with transaction: {}", test_tx);
    
    // Connect to Ethereum node
    let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    
    // Fetch the transaction
    let tx_hash = H256::from_str(test_tx)?;
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
    
    info!("Transaction found in block: {:?}", tx.block_number);
    
    // Initialize Direct simulator
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Convert to the format our simulator expects (same as signal detector)
    let tx_json = serde_json::json!({
        "hash": format!("0x{}", hex::encode(tx.hash.as_bytes())),
        "from": format!("0x{}", hex::encode(tx.from.as_bytes())),
        "to": tx.to.map(|addr| format!("0x{}", hex::encode(addr.as_bytes()))),
        "value": format!("0x{:x}", tx.value),
        "gas": format!("0x{:x}", tx.gas),
        "gasPrice": tx.gas_price.map(|p| format!("0x{:x}", p)),
        "input": format!("0x{}", hex::encode(&tx.input)),
        "nonce": format!("0x{:x}", tx.nonce),
        "v": format!("0x{:x}", tx.v),
        "r": format!("0x{:x}", tx.r),
        "s": format!("0x{:x}", tx.s),
    });
    
    let full_tx = mempool_processor::mempool_fetcher::FullTransaction {
        tx_data: tx_json,
        hash: format!("0x{}", hex::encode(tx.hash.as_bytes())),
        detection_time: std::time::Instant::now(),
        latency_ns: 0,
    };
    
    // Create output file
    let output_filename = format!("/home/nima/code/crypto/logs/mempool/simulator_format_check_{}.log", test_tx);
    let mut output_file = File::create(&output_filename)?;
    
    writeln!(output_file, "Simulator Output Format Check")?;
    writeln!(output_file, "=============================")?;
    writeln!(output_file, "Transaction: {}", test_tx)?;
    writeln!(output_file, "Timestamp: {}\n", chrono::Utc::now())?;
    
    // Try simulation with call trace (same as signal detector)
    info!("Running simulation with call trace...");
    match simulator.simulate_with_call_trace(&full_tx).await {
        Ok(result) => {
            writeln!(output_file, "✅ SIMULATION SUCCESSFUL\n")?;
            writeln!(output_file, "Total addresses: {}\n", result.detailed_changes.len())?;
            
            // Check the structure of the first few entries
            for (i, (address, changes)) in result.detailed_changes.iter().take(3).enumerate() {
                let checksum_addr = mempool_processor::common::address::alloy_address_to_checksum(*address);
                
                writeln!(output_file, "{}. Address: {}", i + 1, checksum_addr)?;
                writeln!(output_file, "   eth_net: {}", changes.eth_net)?;
                writeln!(output_file, "   token_net entries: {}", changes.token_net.len())?;
                
                // Check if token_net contains detailed info or just net amounts
                for (token_key, amount) in &changes.token_net {
                    writeln!(output_file, "     token_net['{}'] = {}", token_key, amount)?;
                }
                
                // Try to access detailed movements if they exist
                writeln!(output_file, "   Type information:")?;
                writeln!(output_file, "     eth_net type: {}", std::any::type_name_of_val(&changes.eth_net))?;
                writeln!(output_file, "     token_net type: {}", std::any::type_name_of_val(&changes.token_net))?;
                
                // Check if there's a movements field (this would indicate the detailed format)
                writeln!(output_file, "   Raw debug format:")?;
                writeln!(output_file, "     {:?}", changes)?;
                
                writeln!(output_file)?;
            }
            
            // Compare with the Python format you want
            writeln!(output_file, "\n" )?;
            writeln!(output_file, "EXPECTED PYTHON FORMAT:")?;
            writeln!(output_file, "======================")?;
            writeln!(output_file, "'0xAddress': {{")?;
            writeln!(output_file, "  'token_net': {{'0xTokenAddr': amount}},")?;
            writeln!(output_file, "  'eth_net': 0.0,")?;
            writeln!(output_file, "  'movements': {{")?;
            writeln!(output_file, "    'tokens': {{'0xTokenAddr': {{'in': OrderedDict(), 'out': OrderedDict()}}}},")?;
            writeln!(output_file, "    'denom': {{'in': OrderedDict(), 'out': OrderedDict()}}")?;
            writeln!(output_file, "  }}")?;
            writeln!(output_file, "}}")?;
            writeln!(output_file, "\nOUR CURRENT FORMAT:")?;
            writeln!(output_file, "==================")?;
            if let Some((first_addr, first_changes)) = result.detailed_changes.iter().next() {
                let addr_str = mempool_processor::common::address::alloy_address_to_checksum(*first_addr);
                writeln!(output_file, "'{}': {{", addr_str)?;
                writeln!(output_file, "  'eth_net': {},", first_changes.eth_net)?;
                writeln!(output_file, "  'token_net': {:?}", first_changes.token_net)?;
                writeln!(output_file, "  // movements field: NOT ACCESSIBLE IN CURRENT STRUCTURE")?;
                writeln!(output_file, "}}")?;
            }
            
        }
        Err(e) => {
            writeln!(output_file, "❌ SIMULATION FAILED")?;
            writeln!(output_file, "Error: {}", e)?;
        }
    }
    
    info!("✅ Format check complete. Output: {}", output_filename);
    println!("Output file: {}", output_filename);
    
    Ok(())
}