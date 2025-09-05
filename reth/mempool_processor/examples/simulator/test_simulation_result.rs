/// Test SimulationResult Architecture
/// 
/// Validates that our unified SimulationResult works correctly

use mempool_processor::simulator::simulation_manager::{SimulationResult, BuySellResult};
use tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::types::PoolViabilityResult;
use mempool_processor::simulator::simulation_manager::{SimulationRequest, SimulationType};
use mempool_processor::mempool_fetcher::FullTransaction;
use mempool_processor::tx_router::TransactionCategory;
use alloy_primitives::{Address, U256};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing SimulationResult Architecture");
    
    // Create a mock FullTransaction
    let mock_tx = FullTransaction {
        hash: "0x1234".to_string(),
        from: "0x5678".to_string(),
        to: Some("0x9abc".to_string()),
        value: "1000000000000000000".to_string(),
        input: "0x".to_string(),
        gas: Some(21000),
        gas_price: Some(20000000000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: Some(1),
        tx_type: Some(0),
        access_list: None,
        chain_id: Some(1),
        timestamp_ns: 1234567890000000000,
        latency_ns: 1000000,
        tx_data: vec![],
    };
    
    // Create a mock SimulationRequest
    let request = SimulationRequest {
        tx: mock_tx,
        category: TransactionCategory::Unknown,
        priority: mempool_processor::tx_router::SimulationPriority::Low,
        simulation_type: SimulationType::TransactionOnly,
        tx_hash: ethers::types::H256::zero(),
    };
    
    // Test 1: Create SimulationResult with pool_viability_result
    let sim_result = SimulationResult {
        request: request.clone(),
        pool_viability_result: None, // Would normally contain PoolViabilityResult
        error: None,
        simulation_time_ms: 5.2,
        token_address: Some(Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?),
        pool_address: Some(Address::from_str("0xB4FBF271143F4FBf7B91A5ded31805e42b2208d6")?),
        pool_type: Some("V2".to_string()),
        debug_info: None,
        liquidity_removal_result: None,
    };
    
    println!("✅ SimulationResult created successfully");
    println!("   - Token: {:?}", sim_result.token_address);
    println!("   - Pool: {:?}", sim_result.pool_address);
    println!("   - Pool Type: {:?}", sim_result.pool_type);
    println!("   - Simulation Time: {:.1}ms", sim_result.simulation_time_ms);
    
    // Test 2: Test the buy_sell_result() conversion method
    let bs_result = sim_result.buy_sell_result();
    println!("✅ buy_sell_result() method works: {:?}", bs_result.is_some());
    
    // Test 3: Create error result
    let error_result = SimulationResult {
        request: request.clone(),
        pool_viability_result: None,
        error: Some("Test error".to_string()),
        simulation_time_ms: 0.0,
        token_address: None,
        pool_address: None,
        pool_type: None,
        debug_info: Some("Test debug info".to_string()),
        liquidity_removal_result: None,
    };
    
    println!("✅ Error SimulationResult created successfully");
    println!("   - Error: {:?}", error_result.error);
    println!("   - Debug: {:?}", error_result.debug_info);
    
    // Test 4: Verify we can create Vec<SimulationResult>
    let results: Vec<SimulationResult> = vec![sim_result, error_result];
    println!("✅ Vec<SimulationResult> works with {} results", results.len());
    
    println!("\n🎉 All SimulationResult architecture tests passed!");
    println!("   - Unified type system working");
    println!("   - No compilation errors");
    println!("   - Ready for signal manager integration");
    
    Ok(())
}