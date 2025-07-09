use mempool_processor::signal_engine::FunctionDetector;

fn main() {
    // Create a new function detector
    let detector = FunctionDetector::new();
    
    // Example transaction data with different function selectors
    let test_cases = vec![
        // Uniswap V2 removeLiquidity
        ("0xe8e33700000000000000000000000000000000000000000000000000000000000000000a", "Uniswap V2 removeLiquidity"),
        // Uniswap V3 decreaseLiquidity
        ("0x0c49ccbe000000000000000000000000000000000000000000000000000000000000001234", "Uniswap V3 decreaseLiquidity"),
        // Curve remove_liquidity
        ("0x5b36389c000000000000000000000000000000000000000000000000000000000000dead", "Curve remove_liquidity"),
        // Unknown function
        ("0x12345678000000000000000000000000000000000000000000000000000000000000beef", "Unknown function"),
        // Too short data
        ("0x1234", "Invalid data"),
    ];
    
    println!("Testing Function Detector:\n");
    
    for (input_data, description) in test_cases {
        println!("Testing: {}", description);
        println!("Input: {}", input_data);
        
        let signals = detector.detect_signals(input_data);
        
        if signals.is_empty() {
            println!("No signals detected");
        } else {
            for signal in signals {
                println!("Detected Signal:");
                println!("  Type: {:?}", signal.signal_type);
                println!("  Function: {}", signal.function_name);
                println!("  Protocol: {}", signal.protocol);
                println!("  Severity: {}", signal.severity);
                println!("  Selector: {}", signal.selector);
            }
        }
        println!();
    }
    
    // Print statistics
    println!("\n{}", detector.get_stats());
}