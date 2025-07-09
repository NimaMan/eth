use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::mempool_fetcher::NonBlockingTransaction;
use ethers::types::U256;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    println!("Function Detector Example\n");
    
    // Create a new function detector
    let detector = FunctionDetector::new();
    
    // Example transactions with different function selectors
    let test_cases = vec![
        (
            vec![0x02, 0x75, 0x1c, 0xec], // removeLiquidityETH
            "0xabc123",
            vec![0x11; 20],
            Some(vec![0x7a, 0x25, 0x0d, 0x56, 0x30, 0xb4, 0xcf, 0x53, 0x97, 0x39, 0xdf, 0x2c, 0x5d, 0xac, 0xb4, 0xc6, 0x59, 0xf2, 0x48, 0x8d]),
            "Uniswap V2 removeLiquidityETH"
        ),
        (
            vec![0xc9, 0x56, 0x7b, 0xf9], // openTrading
            "0xdef456",
            vec![0x22; 20],
            Some(vec![0x33; 20]),
            "Token openTrading"
        ),
        (
            vec![0x12, 0x34, 0x56, 0x78], // Unknown function
            "0x789abc",
            vec![0x44; 20],
            Some(vec![0x55; 20]),
            "Unknown function"
        ),
        (
            vec![0x12], // Too short
            "0x000000",
            vec![0x66; 20],
            Some(vec![0x77; 20]),
            "Invalid input data"
        ),
    ];
    
    println!("Testing Function Detection:\n");
    
    for (input, hash, from, to, description) in test_cases {
        println!("Testing: {}", description);
        println!("  Hash: {}", hash);
        println!("  Selector: {}", if input.len() >= 4 { 
            format!("0x{}", hex::encode(&input[0..4]))
        } else {
            "Invalid".to_string()
        });
        
        // Create a mock transaction
        let tx = NonBlockingTransaction {
            hash: hash.to_string(),
            data: serde_json::Value::Null, // Not used in optimized version
            detection_ns: 1000, // 1 microsecond
            from,
            to,
            input,
            value: U256::zero(),
            gas_price: Some(U256::from(20_000_000_000u64)), // 20 gwei
        };
        
        // Detect function
        detector.detect_from_ipc(&tx);
        
        println!();
    }
    
    // Print statistics
    let stats = detector.get_stats();
    println!("\nFunction Detection Statistics:");
    println!("  Total checked: {}", stats.total_checked);
    println!("  Liquidity removals: {}", stats.liquidity_removals);
    println!("  Trading enabled: {}", stats.trading_enabled);
    println!("  Other functions: {}", stats.other_functions);
}