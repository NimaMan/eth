#!/usr/bin/env python3
"""
Batch Comparison Demo

This script demonstrates how to perform batch comparisons
between Rust and Python transaction processing.
"""

import subprocess
import time

def run_rust_batch_comparison():
    """Run the Rust batch comparison tool"""
    print("🦀 Running Rust Batch Comparison Tool")
    print("=====================================")
    
    # Example transaction hashes (you can replace with real ones)
    test_transactions = [
        "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
        "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b",
    ]
    
    try:
        # Run with predefined test transactions
        print("📝 Running with 10 predefined test transactions:")
        result = subprocess.run([
            "cargo", "run", "--bin", "batch_comparison", "--", 
            "--test-10", "--detailed"
        ], capture_output=True, text=True, timeout=120)
        
        if result.returncode == 0:
            print(result.stdout)
        else:
            print("❌ Error running batch comparison:")
            print(result.stderr)
            
        print("\n" + "="*60)
        print("📝 Running with specific transactions:")
        
        # Run with specific transactions
        cmd = ["cargo", "run", "--bin", "batch_comparison", "--"] + test_transactions + ["--detailed"]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        
        if result.returncode == 0:
            print(result.stdout)
        else:
            print("❌ Error running specific transaction comparison:")
            print(result.stderr)
            
    except subprocess.TimeoutExpired:
        print("⏰ Comparison timed out (this is expected if services are not running)")
    except Exception as e:
        print(f"❌ Error: {e}")

def show_api_usage():
    """Show how to use the batch comparison API directly"""
    print("\n🔧 API Usage Examples")
    print("====================")
    
    print("""
## 1. Basic Batch Comparison

```rust
use revm_tx_simulator_lib::process_tx::batch_compare_with_python;

let tx_hashes = vec![
    "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060".to_string(),
    "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b".to_string(),
    // ... up to 10 or more transactions
];

let results = batch_compare_with_python(
    tx_hashes,
    "http://127.0.0.1:8545",        // Reth RPC URL
    Some("http://127.0.0.1:18000")  // Python service URL
).await?;

// Analyze results
let successful = results.iter().filter(|r| r.matches).count();
let total = results.len();
println!("Success rate: {:.1}%", (successful as f64 / total as f64) * 100.0);
```

## 2. Performance Analysis

```rust
let mut rust_times = Vec::new();
let mut python_times = Vec::new();

for result in &results {
    rust_times.push(result.rust_processing_time_ms);
    python_times.push(result.python_processing_time_ms);
}

let avg_rust: f64 = rust_times.iter().sum::<f64>() / rust_times.len() as f64;
let avg_python: f64 = python_times.iter().sum::<f64>() / python_times.len() as f64;
let speedup = avg_python / avg_rust;

println!("Average Rust time: {:.1}ms", avg_rust);
println!("Average Python time: {:.1}ms", avg_python);
println!("Speedup factor: {:.1}x", speedup);
```

## 3. Error Analysis

```rust
for result in &results {
    if !result.matches {
        println!("Transaction {} failed:", result.tx_hash);
        for diff in &result.differences {
            println!("  • {}: {}", diff.field, diff.description);
        }
    }
}
```

## 4. CLI Tools Available

### Compare 10 Test Transactions
```bash
cargo run --bin batch_comparison -- --test-10
```

### Compare Specific Transactions  
```bash
cargo run --bin batch_comparison -- 0x5c504ed... 0x7b944d9... 0xf7bd63f...
```

### Detailed Output
```bash
cargo run --bin batch_comparison -- --test-10 --detailed
```

### Using the Python Comparison Tool
```bash
# Single transaction
cargo run --bin python_comparison -- 0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060

# Batch of transactions
cargo run --bin python_comparison -- --batch 0x5c504ed... 0x7b944d9... 0xf7bd63f...

# Test examples
cargo run --bin python_comparison -- --test-examples
```
""")

def show_expected_output():
    """Show what the expected output looks like"""
    print("\n📊 Expected Output Format")
    print("=========================")
    
    print("""
🔍 Batch Transaction Comparison
Comparing 10 transactions between Rust and Python implementations
📊 Rust RPC: http://127.0.0.1:8545
🐍 Python Service: http://127.0.0.1:18000

📊 Batch Comparison Summary
════════════════════════════
📈 Match Results:
   Total transactions: 10
   ✅ Successful matches: 8
   ❌ Failed matches: 2
   📊 Success rate: 80.0%

⚡ Performance Results:
   🦀 Total Rust time: 45.2ms
   🐍 Total Python time: 156.8ms
   🦀 Average Rust time: 4.5ms
   🐍 Average Python time: 15.7ms
   🚀 Speedup factor: 3.5x

💯 Overall Performance: Good

⏱️  Total wall time: 89.3ms
🔄 Concurrency efficiency: 2.3x

📋 Detailed Results
═══════════════════
 1. ✅ MATCH 0x5c504ed4... (Rust: 3.2ms, Python: 12.4ms)
 2. ✅ MATCH 0x7b944d90... (Rust: 4.1ms, Python: 18.9ms)
 3. ❌ MISMATCH 0xf7bd63f3... (Rust: 5.7ms, Python: 14.2ms)
    Differences:
      • 0x1234.eth_net: ETH net change mismatch
      • 0x1234.token_net.USDC: Token amount mismatch
 ...
""")

def main():
    """Main demo function"""
    print("🔍 Batch Transaction Comparison Demo")
    print("====================================")
    print()
    
    print("This demo shows how to compare multiple transactions (like 10)")
    print("between Rust and Python implementations for validation.")
    print()
    
    # Show API usage first
    show_api_usage()
    
    # Show expected output
    show_expected_output()
    
    # Ask if user wants to run actual comparison
    try:
        response = input("\n🚀 Run actual batch comparison? (y/n): ").strip().lower()
        if response == 'y':
            run_rust_batch_comparison()
        else:
            print("✅ Demo complete! Use the commands above to run comparisons when services are available.")
    except KeyboardInterrupt:
        print("\n👋 Demo interrupted")

if __name__ == "__main__":
    main()