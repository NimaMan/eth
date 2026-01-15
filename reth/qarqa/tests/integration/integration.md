# Integration Testing Documentation

## Overview

Integration tests verify that all QARQA components work together correctly, testing the complete pipeline from data ingestion through visualization output. This document specifies end-to-end testing scenarios using real blockchain data.

## What We Are Testing

### 1. Full Pipeline Integration

#### Data Flow Pipeline
- **Database → Simulation → Analysis → Network → API**
- **Real transaction processing**: Mainnet transactions
- **Component interaction**: Proper data passing
- **Error propagation**: Failures handled gracefully
- **Performance targets**: End-to-end SLAs

#### Cross-Component Features
- **Transaction enrichment**: DB + RPC data
- **State reconstruction**: Historical state access
- **Cache coordination**: Shared cache usage
- **Resource sharing**: Connection pools
- **Configuration consistency**: Unified config

### 2. Real Transaction Scenarios

#### Transaction Types
- **Simple ETH transfers**: Basic fund movement
- **Token transfers**: ERC20 movements
- **DeFi interactions**: Uniswap, Compound, Aave
- **Contract deployments**: Creation with value
- **Failed transactions**: Reverted operations

#### Complex Scenarios
- **Flash loan arbitrage**: Multi-step transactions
- **DEX aggregator trades**: 1inch, Matcha routing
- **Liquidations**: Collateral and debt handling
- **Governance actions**: DAO operations
- **Bridge transactions**: Cross-chain movements

### 3. Database Integration

#### Data Consistency
- **Transaction completeness**: All fields populated
- **Block synchronization**: Consistent block data
- **Reorg handling**: Chain reorganization updates
- **Index performance**: Query optimization
- **Data integrity**: Foreign key constraints

#### Historical Analysis
- **Time-range queries**: Historical windows
- **Address history**: Complete transaction list
- **State at block**: Point-in-time state
- **Incremental updates**: New block handling
- **Archive data**: Deep history access

### 4. Performance and Scale

#### Load Testing
- **Concurrent requests**: Multiple users
- **Batch processing**: Large transaction sets
- **Streaming data**: Real-time updates
- **Memory management**: No leaks
- **CPU utilization**: Efficient processing

#### Stress Testing
- **Peak load**: 10x normal traffic
- **Sustained load**: 24-hour runs
- **Resource exhaustion**: Memory/CPU limits
- **Recovery testing**: After failures
- **Degradation modes**: Graceful handling

### 5. End-to-End Workflows

#### User Workflows
- **Address investigation**: Complete analysis
- **Transaction tracing**: Full fund flow
- **Network exploration**: Interactive discovery
- **Export workflows**: Data extraction
- **Real-time monitoring**: Live updates

#### System Workflows
- **Batch analysis**: Scheduled jobs
- **Alert generation**: Threshold monitoring
- **Report creation**: Automated reports
- **Data synchronization**: Multi-source sync
- **Backup/restore**: Data recovery

## How We Test It

### Full Pipeline Tests

```rust
#[tokio::test]
async fn test_complete_transaction_analysis_pipeline() {
    // Arrange
    let system = TestSystem::start().await;
    let tx_hash = "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006";
    
    // Act - Full pipeline
    // 1. Fetch from database
    let tx = system.database
        .get_transaction(tx_hash)
        .await
        .expect("Transaction should exist");
    
    // 2. Simulate transaction
    let fund_flows = system.simulator
        .simulate_transaction(&tx)
        .await
        .expect("Simulation should succeed");
    
    // 3. Analyze flows
    let analysis = system.analyzer
        .analyze_fund_flows(&[fund_flows])
        .await
        .expect("Analysis should succeed");
    
    // 4. Build network
    let network = system.network_builder
        .build_from_analysis(&analysis)
        .await
        .expect("Network building should succeed");
    
    // 5. Serve via API
    let response = system.api_client
        .get(&format!("/api/v1/transaction/{}/network", tx_hash))
        .send()
        .await
        .expect("API request should succeed");
    
    // Assert
    assert_eq!(response.status(), 200);
    let api_network: NetworkResponse = response.json().await.unwrap();
    assert_eq!(api_network.nodes.len(), network.nodes.len());
    assert_eq!(api_network.edges.len(), network.edges.len());
}
```

### Real Data Validation Tests

```rust
#[tokio::test]
#[ignore] // Requires mainnet data
async fn test_known_defi_transaction_accuracy() {
    // Test against known transaction with expected results
    let test_cases = vec![
        KnownTransaction {
            hash: "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006",
            expected_eth_movements: 8,
            expected_token_movements: 6,
            expected_nodes: 8,
            expected_total_value_usd: 125_432.50,
        },
    ];
    
    for test_case in test_cases {
        let result = analyze_transaction(test_case.hash).await;
        
        assert_eq!(
            result.eth_movements.len(), 
            test_case.expected_eth_movements,
            "ETH movement count mismatch for {}", 
            test_case.hash
        );
        
        assert!(
            (result.total_value_usd - test_case.expected_total_value_usd).abs() < 100.0,
            "Value calculation mismatch for {}",
            test_case.hash
        );
    }
}
```

### Performance Integration Tests

```rust
#[tokio::test]
async fn test_batch_processing_performance() {
    // Arrange
    let system = TestSystem::start().await;
    let transactions = load_test_transactions(1000); // 1000 real txs
    
    // Act
    let start = Instant::now();
    let results = system.process_batch(transactions).await;
    let duration = start.elapsed();
    
    // Assert
    assert_eq!(results.len(), 1000);
    assert!(duration < Duration::from_secs(60)); // < 1 minute for 1000
    
    // Verify memory usage
    let memory_after = get_process_memory();
    assert!(memory_after < 1_000_000_000); // < 1GB
}
```

### Error Recovery Tests

```rust
#[tokio::test]
async fn test_database_failure_recovery() {
    // Arrange
    let mut system = TestSystem::start().await;
    
    // Act - Simulate database failure
    system.database.shutdown().await;
    
    let result = system.api_client
        .get("/api/v1/fund-flow/0xtest")
        .send()
        .await;
    
    // Should fail gracefully
    assert_eq!(result.unwrap().status(), 503);
    
    // Restart database
    system.database.start().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Should recover
    let result = system.api_client
        .get("/api/v1/health")
        .send()
        .await;
    
    assert_eq!(result.unwrap().status(), 200);
}
```

## Test Scenarios

### Transaction Processing Scenarios

1. **Simple Transfers**
   - ETH transfer between EOAs
   - Token transfer (USDC, USDT)
   - Multiple transfers in one tx
   - Failed transfer (revert)
   - Zero value transfer

2. **DeFi Protocols**
   - Uniswap V2/V3 swaps
   - Compound deposit/borrow
   - Aave flash loans
   - Curve multi-pool swaps
   - Balancer weighted pools

3. **Complex Interactions**
   - MEV arbitrage bundles
   - Liquidation cascades
   - Governance proposals
   - Multi-sig operations
   - Proxy upgrades

### System Integration Scenarios

1. **Data Sources**
   - Database only
   - Database + RPC fallback
   - RPC only (no DB data)
   - Cached data usage
   - Multiple RPC endpoints

2. **Load Patterns**
   - Steady state (100 req/min)
   - Burst traffic (1000 req/min)
   - Sustained high load
   - Gradual ramp up
   - Circuit breaker activation

3. **Failure Modes**
   - Database connection loss
   - RPC endpoint timeout
   - Memory exhaustion
   - CPU saturation
   - Network partition

### End-to-End User Journeys

1. **Investigation Flow**
   ```
   User enters address →
   System fetches history →
   Analyzes recent transactions →
   Builds relationship network →
   Displays interactive graph →
   User explores connections →
   Exports data
   ```

2. **Monitoring Flow**
   ```
   User sets up alerts →
   System monitors addresses →
   Detects matching transaction →
   Simulates and analyzes →
   Triggers alert →
   User receives notification →
   Views detailed analysis
   ```

## Test Data

### Real Mainnet Transactions
```rust
pub struct TestTransaction {
    pub hash: &'static str,
    pub block: u64,
    pub transaction_type: &'static str,
    pub complexity: Complexity,
    pub expected_results: ExpectedResults,
}

pub const TEST_TRANSACTIONS: &[TestTransaction] = &[
    TestTransaction {
        hash: "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
        block: 46147,
        transaction_type: "First Ethereum Transaction",
        complexity: Complexity::Simple,
        expected_results: ExpectedResults {
            eth_movements: 1,
            gas_used: 21000,
            status: true,
        },
    },
    // ... more test transactions
];
```

### Test Database Setup
```sql
-- Ensure test database has real data
COPY transactions FROM '/data/mainnet_transactions.csv';
COPY blocks FROM '/data/mainnet_blocks.csv';
COPY address_transactions FROM '/data/mainnet_participants.csv';

-- Create necessary indexes
CREATE INDEX CONCURRENTLY idx_test_address_tx ON address_transactions(address);
CREATE INDEX CONCURRENTLY idx_test_tx_hash ON transactions(hash);
```

## Expected Test Outcomes

### Functional Requirements
- All components integrate successfully
- Data flows correctly between components
- Real transactions process accurately
- Error handling works end-to-end
- Results match expected values

### Performance Requirements
- End-to-end latency < 2 seconds
- Throughput > 100 tx/second
- Memory usage < 2GB under load
- No memory leaks in 24-hour run
- CPU usage < 80% sustained

### Reliability Requirements
- 99.9% success rate for valid inputs
- Graceful degradation under load
- Recovery from component failures
- No data loss or corruption
- Consistent results across runs

## Running Integration Tests

```bash
# Setup test environment
./scripts/setup_integration_env.sh

# Run all integration tests
cargo test --test integration_tests --features integration

# Run specific scenario
cargo test --test integration_tests test_defi_scenarios

# Run performance tests
cargo test --test integration_tests performance_ -- --ignored

# Run with real mainnet data
MAINNET_RPC_URL=https://eth.llamarpc.com cargo test --test integration_tests mainnet_

# Long-running stability tests
cargo test --test integration_tests stability_ -- --ignored --test-threads=1
```

## Test Environment Requirements

### Infrastructure
- PostgreSQL 14+ with mainnet data subset
- Local Ethereum node or reliable RPC
- Redis for caching layer
- 16GB RAM minimum
- 4+ CPU cores

### Configuration
```toml
[test]
database_url = "postgresql://localhost/qarqa_test"
rpc_endpoints = ["http://localhost:8545", "https://eth.llamarpc.com"]
cache_url = "redis://localhost:6379"
max_concurrent_requests = 100
request_timeout_seconds = 30
```

## Continuous Integration

### CI Pipeline Stages
1. **Unit Tests**: Component isolation
2. **Integration Tests**: Component interaction
3. **System Tests**: Full pipeline
4. **Performance Tests**: Benchmarks
5. **Stability Tests**: Long-running

### Test Data Management
- Maintain curated test transaction set
- Update quarterly with new patterns
- Version control test expectations
- Document any deviations
- Automated data validation

## Test Maintenance

- Review integration points monthly
- Update test data quarterly
- Monitor test execution time
- Document flaky tests
- Maintain test environment scripts