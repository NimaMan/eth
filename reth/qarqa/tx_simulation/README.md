# QARQA Transaction Simulation

## 🎯 **Overview**

The Transaction Simulation module analyzes Ethereum transactions to extract fund flows, state changes, and economic patterns. It provides both lightweight development simulation and full REVM-based transaction execution for comprehensive analysis.

## 🏗️ **Architecture**

### **Module Structure**
```
tx_simulation/
├── src/
│   ├── lib.rs           # Public API exports
│   ├── simulator.rs     # Core simulation engine
│   ├── fund_flows.rs    # Fund flow analysis
│   └── state_changes.rs # State change tracking
├── tests/               # Unit and integration tests
└── Cargo.toml           # Simulation dependencies
```

### **Core Principles**
- **Accurate Analysis**: Extract real fund flows without mock data
- **Performance**: Fast simulation for real-time analysis
- **Flexibility**: Support both lightweight and full EVM simulation
- **Type Safety**: Leverages core_types for consistent data structures

## 🔧 **Simulation Modes**

### **1. Development Simulator (Lightweight)**
Fast analysis focusing on fund flows without full EVM execution.

```rust
use qarqa_tx_simulation::DevelopmentSimulator;

let simulator = DevelopmentSimulator::new();

// Analyze transaction fund flows
let result = simulator
    .simulate_transaction(&transaction)
    .await?;

println!("Fund flows extracted: {}", result.fund_flows.len());
```

**Features:**
- **Fast Execution**: No EVM overhead
- **Fund Flow Extraction**: Direct analysis of transfers
- **State Change Calculation**: Net balance changes
- **Memory Efficient**: Minimal resource usage

### **2. REVM Simulator (Full Execution)**
Complete EVM simulation for comprehensive analysis.

```rust
use qarqa_tx_simulation::RevmSimulator;

let simulator = RevmSimulator::new()
    .with_fork_url("https://eth-mainnet.g.alchemy.com/v2/...")
    .with_block_number(18_500_000)
    .build();

// Full transaction simulation
let result = simulator
    .simulate_transaction_complete(&transaction)
    .await?;

// Access complete execution trace
for trace in result.execution_trace {
    println!("Call: {} -> {}", trace.from, trace.to);
}
```

**Features:**
- **Complete EVM Execution**: Full transaction simulation
- **State Overrides**: Custom state modifications
- **Execution Traces**: Detailed call traces
- **Gas Analysis**: Precise gas consumption tracking

## 📊 **Core Components**

### **1. Fund Flow Analyzer (`fund_flows.rs`)**
Extracts and analyzes fund movements from transaction data.

```rust
use qarqa_tx_simulation::FundFlowAnalyzer;

let analyzer = FundFlowAnalyzer::new()
    .with_min_eth_amount(0.001)  // Filter small amounts
    .with_stablecoin_detection(true)
    .with_token_classification(true);

// Analyze transaction fund flows
let fund_flows = analyzer
    .analyze_transaction(&transaction)
    .await?;

// Categorize flows
for flow in fund_flows {
    match flow.classify_flow_type() {
        FlowType::DirectTransfer => println!("Direct ETH transfer"),
        FlowType::TokenSwap => println!("Token swap detected"),
        FlowType::LiquidityProvision => println!("LP operation"),
        FlowType::Arbitrage => println!("Arbitrage opportunity"),
    }
}
```

**Analysis Capabilities:**
- **ETH Flow Detection**: Direct and internal transfers
- **Token Flow Analysis**: ERC-20 token movements
- **Pattern Recognition**: DEX swaps, LP operations, arbitrage
- **Value Classification**: High-value vs routine transactions

### **2. State Change Analyzer (`state_changes.rs`)**
Tracks balance changes and identifies significant state modifications.

```rust
use qarqa_tx_simulation::StateChangeAnalyzer;

let analyzer = StateChangeAnalyzer::new()
    .with_significance_threshold(0.1) // 0.1 ETH minimum
    .with_top_movers_count(10);

// Analyze state changes
let state_changes = analyzer
    .analyze_transaction(&transaction)
    .await?;

// Get top movers
let top_gainers = state_changes.get_top_gainers(5);
let top_losers = state_changes.get_top_losers(5);

// Calculate net flows
let net_eth_flow = state_changes.calculate_net_eth_flow();
let net_token_flows = state_changes.calculate_net_token_flows();
```

**State Analysis Features:**
- **Balance Tracking**: ETH and token balance changes
- **Significance Filtering**: Focus on meaningful changes
- **Top Movers**: Identify biggest gainers/losers
- **Net Flow Calculation**: Overall transaction impact

### **3. Simulation Engine (`simulator.rs`)**
Core simulation logic with support for multiple execution modes.

```rust
use qarqa_tx_simulation::{Simulator, SimulationConfig};

let config = SimulationConfig::new()
    .with_mode(SimulationMode::Development)
    .with_gas_limit(10_000_000)
    .with_state_overrides(state_overrides);

let simulator = Simulator::new(config);

// Simulate with custom configuration
let result = simulator
    .simulate_with_config(&transaction, &config)
    .await?;

// Access simulation results
println!("Gas used: {}", result.gas_used);
println!("Status: {:?}", result.status);
println!("Fund flows: {}", result.fund_flows.len());
```

## 🎨 **Usage Examples**

### **Basic Fund Flow Analysis**
```rust
use qarqa_tx_simulation::*;
use qarqa_core_types::*;

#[tokio::main]
async fn main() -> QarqaResult<()> {
    // Create development simulator
    let simulator = DevelopmentSimulator::new();
    
    // Example transaction
    let transaction = Transaction {
        hash: "0xabc123...".parse()?,
        from_address: "0x742d35Cc...".parse()?,
        to_address: Some("0x1234567...".parse()?),
        value: eth_to_wei(1.5),
        // ... other fields
        internal_transfers: vec![
            EthMovement {
                from_address: "0x742d35Cc...".parse()?,
                to_address: "0x987fcdeb...".parse()?,
                amount: eth_to_wei(0.5),
                movement_type: MovementType::Call,
            }
        ],
        token_transfers: vec![
            TokenMovement {
                token_address: "0xA0b86a33...".parse()?, // USDC
                from_address: "0x742d35Cc...".parse()?,
                to_address: "0x987fcdeb...".parse()?,
                amount: U256::from(1000_000_000), // 1000 USDC
                token_symbol: Some("USDC".to_string()),
                token_decimals: Some(6),
            }
        ],
        // ... other fields
    };
    
    // Simulate transaction
    let result = simulator
        .simulate_transaction(&transaction)
        .await?;
    
    // Analyze results
    println!("Simulation Results:");
    println!("- Status: {:?}", result.status);
    println!("- Gas Used: {}", result.gas_used);
    println!("- Fund Flows: {}", result.fund_flows.len());
    
    // Examine fund flows
    for (i, flow) in result.fund_flows.iter().enumerate() {
        println!("Flow {}: {} -> {}", 
            i + 1,
            format_address(&flow.from_address),
            format_address(&flow.to_address)
        );
        println!("  ETH: {} ETH", flow.eth_amount);
        
        for token_flow in &flow.token_flows {
            println!("  Token: {} {}", 
                token_flow.amount, 
                token_flow.symbol
            );
        }
    }
    
    Ok(())
}
```

### **State Change Analysis**
```rust
use qarqa_tx_simulation::*;

async fn analyze_whale_movements(
    transaction: &Transaction
) -> QarqaResult<Vec<StateChange>> {
    let analyzer = StateChangeAnalyzer::new()
        .with_significance_threshold(10.0) // 10 ETH minimum
        .with_whale_threshold(1000.0);     // 1000 ETH whale level
    
    let state_changes = analyzer
        .analyze_transaction(transaction)
        .await?;
    
    // Filter for whale movements
    let whale_movements: Vec<StateChange> = state_changes
        .changes
        .into_iter()
        .filter(|change| {
            change.eth_change.abs() > 1000.0 || 
            change.total_usd_change > 1_000_000.0
        })
        .collect();
    
    // Log significant movements
    for change in &whale_movements {
        tracing::info!(
            "Whale movement detected: {} changed by {} ETH (${:.2})",
            format_address(&change.address),
            change.eth_change,
            change.total_usd_change
        );
    }
    
    Ok(whale_movements)
}
```

### **Arbitrage Detection**
```rust
use qarqa_tx_simulation::*;

async fn detect_arbitrage_opportunity(
    transaction: &Transaction
) -> QarqaResult<Option<ArbitragePattern>> {
    let analyzer = FundFlowAnalyzer::new()
        .with_arbitrage_detection(true)
        .with_min_profit_threshold(0.01); // 0.01 ETH minimum profit
    
    let fund_flows = analyzer
        .analyze_transaction(transaction)
        .await?;
    
    // Look for circular flow patterns
    let mut pools_involved = HashSet::new();
    let mut total_eth_in = 0.0;
    let mut total_eth_out = 0.0;
    
    for flow in &fund_flows {
        // Check if involves known DEX contracts
        if is_dex_contract(&flow.to_address) {
            pools_involved.insert(flow.to_address);
            total_eth_out += flow.eth_amount;
        }
        
        if is_dex_contract(&flow.from_address) {
            pools_involved.insert(flow.from_address);
            total_eth_in += flow.eth_amount;
        }
    }
    
    // Detect arbitrage pattern
    if pools_involved.len() >= 2 && total_eth_in > total_eth_out {
        let profit = total_eth_in - total_eth_out;
        
        return Ok(Some(ArbitragePattern {
            pools: pools_involved.into_iter().collect(),
            profit_eth: profit,
            transaction_hash: transaction.hash,
            strategy_type: ArbitrageStrategy::CrossDex,
        }));
    }
    
    Ok(None)
}
```

### **Batch Transaction Analysis**
```rust
use qarqa_tx_simulation::*;

async fn analyze_batch_transactions(
    transactions: Vec<Transaction>
) -> QarqaResult<BatchAnalysisResult> {
    let simulator = DevelopmentSimulator::new();
    let mut results = Vec::new();
    
    // Process transactions in parallel
    let futures: Vec<_> = transactions
        .iter()
        .map(|tx| simulator.simulate_transaction(tx))
        .collect();
    
    let simulation_results = futures::future::try_join_all(futures).await?;
    
    // Aggregate results
    let mut total_gas = 0;
    let mut total_eth_volume = 0.0;
    let mut total_token_transfers = 0;
    
    for result in simulation_results {
        total_gas += result.gas_used;
        
        for flow in result.fund_flows {
            total_eth_volume += flow.eth_amount;
            total_token_transfers += flow.token_flows.len();
        }
        
        results.push(result);
    }
    
    Ok(BatchAnalysisResult {
        individual_results: results,
        total_gas_used: total_gas,
        total_eth_volume,
        total_token_transfers,
        analysis_duration: start_time.elapsed(),
    })
}
```

## 🧪 **Testing**

### **Running Tests**
```bash
cd /home/nima/code/crypto/rust/qarqa/tx_simulation
cargo test
```

### **Test Categories**
- ✅ **Unit Tests**: Core simulation logic, fund flow extraction
- ✅ **Integration Tests**: End-to-end simulation workflows
- ✅ **Performance Tests**: Simulation speed and memory usage
- ✅ **Accuracy Tests**: Comparison with known transaction results

### **Example Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_transfer_simulation() {
        let simulator = DevelopmentSimulator::new();
        
        let transaction = create_simple_transfer(
            "0x742d35Cc...".parse().unwrap(),
            "0x1234567...".parse().unwrap(),
            eth_to_wei(1.0)
        );
        
        let result = simulator
            .simulate_transaction(&transaction)
            .await
            .unwrap();
        
        assert_eq!(result.status, TransactionStatus::Success);
        assert_eq!(result.fund_flows.len(), 1);
        assert_eq!(result.fund_flows[0].eth_amount, 1.0);
    }

    #[tokio::test]
    async fn test_complex_defi_transaction() {
        let simulator = DevelopmentSimulator::new();
        
        let transaction = load_test_transaction("complex_uniswap_v3_swap.json");
        
        let result = simulator
            .simulate_transaction(&transaction)
            .await
            .unwrap();
        
        // Verify complex transaction handling
        assert!(result.fund_flows.len() > 1);
        assert!(result.fund_flows.iter().any(|f| !f.token_flows.is_empty()));
    }

    #[test]
    fn test_fund_flow_analyzer_performance() {
        let analyzer = FundFlowAnalyzer::new();
        let transaction = create_complex_transaction_with_100_transfers();
        
        let start = Instant::now();
        let _flows = analyzer.analyze_transaction_sync(&transaction).unwrap();
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_millis(10)); // < 10ms
    }
}
```

## ⚡ **Performance Optimization**

### **Memory Management**
```rust
// Efficient processing for large transactions
let analyzer = FundFlowAnalyzer::new()
    .with_streaming_mode(true)      // Process transfers incrementally
    .with_memory_limit(100_000_000) // 100MB limit
    .with_batch_size(1000);         // Process 1000 transfers at a time

// Stream processing for memory efficiency
let mut stream = analyzer.analyze_transaction_stream(&transaction).await?;
while let Some(fund_flow) = stream.next().await {
    process_fund_flow(fund_flow?).await;
}
```

### **Parallel Processing**
```rust
// Parallel analysis for batch transactions
use rayon::prelude::*;

let results: Vec<SimulationResult> = transactions
    .par_iter()
    .map(|tx| {
        let simulator = DevelopmentSimulator::new();
        simulator.simulate_transaction_sync(tx).unwrap()
    })
    .collect();
```

### **Caching Strategy**
```rust
// Cache simulation results for repeated analysis
let simulator = DevelopmentSimulator::new()
    .with_cache_size(10000)         // Cache 10k results
    .with_cache_ttl(3600)           // 1 hour TTL
    .with_persistent_cache(true);   // Persist to disk

// Cached simulation (fast on repeated calls)
let result = simulator
    .simulate_transaction_cached(&transaction)
    .await?;
```

## 📈 **Performance Characteristics**

| Operation | Complexity | Typical Performance |
|-----------|------------|-------------------|
| Simple Transfer | O(1) | <1ms |
| Complex DeFi Transaction | O(n) | <10ms for 100 transfers |
| State Change Analysis | O(n) | <5ms for 50 addresses |
| Fund Flow Extraction | O(n log n) | <20ms for 500 transfers |
| Batch Processing | O(n*m) | <100ms for 10 complex transactions |

## 🔗 **Integration with Other Modules**

### **Used By**
- **network_building**: Uses fund flows for network construction
- **api_layer**: Provides simulation results for CLI/API
- **analysis tools**: Real-time transaction analysis

### **Uses**
- **qarqa_core_types**: Transaction, FundFlow, and error types
- **qarqa_data_access**: Historical transaction data (optional)

### **Dependencies**
- **revm**: Ethereum Virtual Machine simulation
- **alloy**: Ethereum types and utilities
- **tokio**: Async runtime
- **tracing**: Structured logging

## 🔄 **Development Guidelines**

### **Adding New Analysis Features**
1. Implement in appropriate module (fund_flows.rs, state_changes.rs)
2. Add comprehensive unit tests
3. Benchmark performance impact
4. Document analysis methodology
5. Update examples and integration tests

### **Simulation Accuracy**
1. Compare results with known transaction outcomes
2. Validate gas usage calculations
3. Test edge cases and error conditions
4. Verify state change calculations

### **Performance Considerations**
1. Profile memory usage for large transactions
2. Optimize hot paths in analysis loops
3. Consider parallelization for batch operations
4. Monitor cache hit rates and effectiveness

This module provides accurate, fast transaction simulation capabilities that power the QARQA analytics system's understanding of complex blockchain interactions.