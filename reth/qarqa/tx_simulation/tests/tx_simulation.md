# Transaction Simulation Testing Documentation

## Overview

The `tx_simulation` module simulates Ethereum transactions using REVM to extract fund flows, internal transfers, and state changes. This document specifies comprehensive testing for simulation accuracy, performance, and edge cases.

## What We Are Testing

### 1. Transaction Simulation Accuracy

#### Simple Transfers
- **Direct ETH transfers**: Sender → Receiver value movement
- **Gas consumption**: Accurate gas usage calculation
- **Failed transfers**: Insufficient balance scenarios
- **Zero value transfers**: Valid but no value movement
- **Self transfers**: Sender = Receiver edge case

#### Complex DeFi Transactions
- **Token swaps**: Uniswap/Sushiswap trade execution
- **Multi-hop swaps**: A → B → C → D token paths
- **Flash loans**: Borrow and repay in same transaction
- **Liquidity operations**: Add/remove liquidity tracking
- **Complex callbacks**: Nested contract interactions

#### Internal Transfers
- **Contract ETH transfers**: transfer() and send() calls
- **CALL operations**: Track value in CALL opcodes
- **DELEGATECALL**: Proper context handling
- **CREATE/CREATE2**: Track value sent to new contracts
- **SELFDESTRUCT**: Beneficiary receives funds

### 2. State Change Tracking

#### Balance Changes
- **ETH balance updates**: Before/after comparisons
- **Token balance updates**: ERC20 balance changes
- **Multi-token transactions**: Track all token movements
- **Gas refunds**: Accurate refund calculations
- **Mining rewards**: Coinbase transfers (if applicable)

#### Account State
- **Nonce increments**: Track account nonce changes
- **Code deployment**: New contract code tracking
- **Storage updates**: Contract storage modifications
- **Account creation**: New account detection
- **Account deletion**: SELFDESTRUCT handling

### 3. Fund Flow Analysis

#### Flow Extraction
- **Direct flows**: Simple A → B transfers
- **Indirect flows**: A → Contract → B patterns
- **Circular flows**: A → B → C → A detection
- **Split flows**: One source, multiple destinations
- **Aggregated flows**: Multiple sources, one destination

#### Flow Classification
- **Transfer types**: Direct, Internal, Gas, Token
- **Flow direction**: In, Out, Both (for intermediaries)
- **Value calculation**: Accurate ETH and token amounts
- **Flow aggregation**: Combine related transfers
- **Duplicate detection**: No double counting

### 4. Error Handling

#### Simulation Failures
- **Reverted transactions**: Proper revert reason extraction
- **Out of gas**: Gas estimation vs actual usage
- **Invalid opcodes**: Handle unknown opcodes gracefully
- **Stack errors**: Stack too deep scenarios
- **State errors**: Invalid state access

#### Recovery Mechanisms
- **Partial simulation**: Extract what's possible
- **Fallback strategies**: RPC when simulation fails
- **Error classification**: Recoverable vs fatal
- **Retry logic**: Transient failure handling
- **Degraded mode**: Basic transfer extraction

### 5. Performance

#### Simulation Speed
- **Simple transfers**: < 1ms simulation time
- **DeFi transactions**: < 10ms for complex txs
- **Batch processing**: 1000 tx/second throughput
- **Memory efficiency**: < 100MB for large transactions
- **CPU utilization**: Efficient opcode execution

#### Caching Strategy
- **State caching**: Reuse account states
- **Result caching**: Cache simulation results
- **Cache invalidation**: Handle state changes
- **Memory bounds**: Limited cache size
- **Hit rate optimization**: > 80% cache hits

## How We Test It

### Unit Tests for Core Logic

```rust
#[tokio::test]
async fn test_simple_eth_transfer_simulation() {
    // Arrange
    let tx = create_test_transaction(
        "0xSender",
        "0xReceiver", 
        eth_to_wei(1.0), // 1 ETH
        21000, // gas limit
        gwei_to_wei(20) // gas price
    );
    
    let simulator = DevelopmentTransactionSimulator::new();
    
    // Act
    let result = simulator.simulate_transaction(&tx).await.unwrap();
    
    // Assert
    assert_eq!(result.eth_movements.len(), 2); // Transfer + Gas
    
    let transfer = &result.eth_movements[0];
    assert_eq!(transfer.from, tx.from_address);
    assert_eq!(transfer.to, tx.to_address.unwrap());
    assert_eq!(transfer.amount, eth_to_wei(1.0));
    assert_eq!(transfer.movement_type, EthMovementType::Direct);
    
    let gas_payment = &result.eth_movements[1];
    assert_eq!(gas_payment.movement_type, EthMovementType::Gas);
}
```

### Integration Tests with REVM

```rust
#[tokio::test]
async fn test_complex_defi_transaction() {
    // Arrange - Real Uniswap transaction
    let tx_hash = "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006";
    let tx = fetch_real_transaction(tx_hash).await;
    
    let simulator = RevmDirectSimulator::new();
    
    // Act
    let result = simulator.simulate_transaction(&tx).await.unwrap();
    
    // Assert - Should extract all movements
    assert!(result.eth_movements.len() > 2);
    assert!(!result.token_movements.is_empty());
    
    // Verify balance conservation
    let total_in = sum_movements_in(&result);
    let total_out = sum_movements_out(&result);
    assert_eq!(total_in, total_out + gas_used);
}
```

### State Change Verification

```rust
#[test]
fn test_state_change_analysis() {
    // Arrange
    let fund_flows = vec![
        FundFlow {
            from: ADDR_A,
            to: ADDR_B,
            amount_eth: 1.0,
            amount_tokens_usd: 0.0,
            transaction_count: 1,
            first_block: 100,
            last_block: 100,
            flow_type: FlowType::DirectTransfer,
        },
        FundFlow {
            from: ADDR_B,
            to: ADDR_C,
            amount_eth: 0.5,
            amount_tokens_usd: 0.0,
            transaction_count: 1,
            first_block: 101,
            last_block: 101,
            flow_type: FlowType::DirectTransfer,
        },
    ];
    
    // Act
    let analyzer = StateChangeAnalyzer::new();
    let state_changes = analyzer.calculate_state_changes(&fund_flows);
    
    // Assert
    assert_eq!(state_changes.len(), 3); // A, B, C
    
    assert_eq!(state_changes[&ADDR_A].net_eth_change(), -1.0);
    assert_eq!(state_changes[&ADDR_B].net_eth_change(), 0.5); // +1.0 - 0.5
    assert_eq!(state_changes[&ADDR_C].net_eth_change(), 0.5);
}
```

### Performance Benchmarks

```rust
#[bench]
fn bench_transaction_simulation(b: &mut Bencher) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let simulator = runtime.block_on(async {
        RevmDirectSimulator::new()
    });
    
    b.iter(|| {
        runtime.block_on(async {
            let tx = create_complex_defi_transaction();
            let _ = simulator.simulate_transaction(&tx).await;
        })
    });
}
```

## Test Cases

### Simple Transaction Tests

1. **ETH Transfers**
   - 0.001 ETH transfer (small)
   - 1000 ETH transfer (large)
   - 0 ETH transfer (valid but empty)
   - Max U256 transfer (overflow test)
   - Insufficient balance transfer

2. **Gas Scenarios**
   - Exact gas used = gas limit
   - Gas refund scenarios
   - Out of gas during execution
   - High gas price impact
   - EIP-1559 transactions

3. **Contract Interactions**
   - ETH sent with contract call
   - Pure view function (no state change)
   - Multiple internal transfers
   - Recursive calls
   - Reentrancy scenarios

### DeFi Transaction Tests

1. **Token Swaps**
   - ETH → Token swap
   - Token → Token swap
   - Multi-hop swap (3+ pools)
   - Slippage scenarios
   - Failed swaps (revert)

2. **Liquidity Operations**
   - Add liquidity (ETH + Token)
   - Remove liquidity (receive both)
   - Single-sided liquidity
   - Impermanent loss scenarios
   - Fee collection

3. **Complex Protocols**
   - Flash loan + arbitrage
   - Yield farming deposits
   - Liquidation transactions
   - Cross-protocol interactions
   - Batch operations

### Edge Case Tests

1. **Unusual Transactions**
   - SELFDESTRUCT operations
   - Contract creation with value
   - Failed contract creation
   - Delegate calls with value
   - Proxy contract interactions

2. **State Edge Cases**
   - First transaction for address
   - Storage collision scenarios
   - Maximum call depth
   - Stack limit approaching
   - Memory expansion limits

3. **Error Conditions**
   - Invalid opcodes
   - Bad jumps
   - Stack underflow/overflow
   - Invalid memory access
   - Arithmetic overflow

## Test Data

### Real Transaction Hashes
```rust
pub const TEST_TRANSACTIONS: &[(&str, &str)] = &[
    // Simple transfers
    ("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060", "First ETH tx"),
    ("0x0c59f431352495d21a398c94b8e30b728e35ce85b67cbd7497906f5300c89a18", "Simple transfer"),
    
    // DeFi transactions  
    ("0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006", "Complex swap"),
    ("0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", "Uniswap V3"),
    
    // Edge cases
    ("0x7a35c56e1d32c3e22c9f6908b38d9db8a376c4306f1c401a9598226bd44a54a8", "Failed tx"),
    ("0x434b529473163ef4fd4a688974ae16728d228857f5b95344585327e88a7d6c3e", "Contract creation"),
];
```

### Test State Setup
```rust
pub fn setup_test_state() -> TestState {
    TestState {
        accounts: vec![
            Account {
                address: "0xSender",
                balance: eth_to_wei(10.0),
                nonce: 5,
            },
            Account {
                address: "0xReceiver",
                balance: eth_to_wei(0.0),
                nonce: 0,
            },
        ],
        contracts: vec![
            Contract {
                address: "0xUniswapRouter",
                code: UNISWAP_BYTECODE,
                storage: default_storage(),
            },
        ],
    }
}
```

## Expected Test Outcomes

### Accuracy Requirements
- ETH transfers: 100% accurate to the wei
- Gas calculation: Within 1% of actual usage
- Token transfers: All transfers detected
- Internal transfers: No missed transfers
- State changes: Exact balance matching

### Performance Targets
- Simple transfer: < 0.5ms simulation
- Token swap: < 5ms simulation  
- Complex DeFi: < 20ms simulation
- Batch processing: > 500 tx/second
- Memory per tx: < 10MB average

### Error Handling
- Revert reason extraction: 100% success
- Partial simulation: Extract direct transfers minimum
- Error recovery: < 3 retry attempts
- Fallback success: > 90% when simulation fails
- No panics: Graceful handling of all errors

## Running the Tests

```bash
# Run all tx_simulation tests
cargo test -p qarqa-tx-simulation

# Run with REVM debugging
RUST_LOG=revm=debug cargo test -p qarqa-tx-simulation

# Run integration tests
cargo test -p qarqa-tx-simulation --features integration-tests -- --ignored

# Run benchmarks
cargo bench -p qarqa-tx-simulation

# Run with specific test
cargo test -p qarqa-tx-simulation test_complex_defi_transaction -- --exact
```

## Test Environment

### REVM Configuration
```rust
pub fn test_revm_config() -> RevmConfig {
    RevmConfig {
        chain_id: 1, // Mainnet
        spec_id: SpecId::LONDON,
        gas_limit: 30_000_000,
        disable_balance_check: false,
        disable_nonce_check: false,
    }
}
```

### Mock State Provider
```rust
impl StateProvider for TestStateProvider {
    fn basic(&self, address: Address) -> AccountInfo {
        // Return test account state
    }
    
    fn storage(&self, address: Address, slot: U256) -> U256 {
        // Return test storage values
    }
}
```

## Test Maintenance

- Update test transactions quarterly
- Profile simulation performance monthly
- Add new DeFi protocol tests as needed
- Document any simulation discrepancies
- Keep REVM version synchronized