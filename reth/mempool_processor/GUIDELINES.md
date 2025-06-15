# Mempool Processor Production Guidelines

## Overview

The mempool processor is a high-performance, production-ready system for monitoring Ethereum transactions in real-time. This document provides guidelines for development, testing, and production deployment.

## Development Guidelines

### Code Quality Standards

1. **No Mock Data**: All code must work with real blockchain data
   - Connect to actual Ethereum nodes (local or remote)
   - Use real transaction hashes and addresses
   - Test against mainnet or testnet data

2. **Error Handling**: Every operation that can fail must be handled
   ```rust
   // ✅ Good
   match provider.get_transaction(hash).await {
       Ok(Some(tx)) => process_transaction(tx),
       Ok(None) => warn!("Transaction not found"),
       Err(e) => error!("Failed to fetch transaction: {}", e),
   }
   
   // ❌ Bad
   let tx = provider.get_transaction(hash).await.unwrap();
   ```

3. **Resource Management**: Prevent memory exhaustion
   - Use bounded channels: `mpsc::channel(10000)`
   - Implement cache eviction strategies
   - Use semaphores for concurrency control

4. **Performance**: Optimize for low latency
   - No artificial delays or sleep statements
   - Use concurrent processing where appropriate
   - Batch operations when possible

### Security Requirements

1. **Input Validation**: Always validate external inputs
   ```rust
   if tx.hash.len() != 32 {
       return Err(eyre::eyre!("Invalid hash length"));
   }
   ```

2. **Bounds Checking**: Prevent integer overflows
   ```rust
   let value_u64 = tx.value.min(U256::from(u64::MAX)).as_u64();
   ```

3. **Stale Data**: Never make decisions on outdated information
   ```rust
   if pool_state.is_stale(MAX_AGE) {
       continue; // Skip stale data
   }
   ```

## Testing Guidelines

### Unit Tests
Every module must have comprehensive unit tests:
```bash
cargo test --lib
```

### Integration Tests
Test against real Ethereum nodes:
```bash
# Start local node first
cargo test --test integration_test
```

### Performance Tests
Monitor transaction processing metrics:
```bash
cd tools/python/core
./run_timing_analysis.sh --no-plots
```

### Example Programs
All examples must be functional:
```bash
cargo run --example mempool_monitor
cargo run --example pool_rug_detector
```

## Production Deployment

### Prerequisites

1. **Infrastructure**:
   - Ethereum node with WebSocket enabled
   - PostgreSQL database for persistence
   - Sufficient memory (8GB+ recommended)
   - SSD storage for state cache

2. **Configuration**:
   ```bash
   export WS_RPC_URL="ws://localhost:8546"
   export HTTP_RPC_URL="http://localhost:8545"
   export THRESHOLD_WEI="1000000000000000000"  # 1 ETH
   ```

### Deployment Steps

1. **Build Release Binary**:
   ```bash
   cargo build --release
   ```

2. **Run with Monitoring**:
   ```bash
   RUST_LOG=info ./target/release/mempool_processor
   ```

3. **Health Checks**:
   - Monitor WebSocket connection status
   - Track transaction processing rate
   - Check memory usage regularly

### Performance Tuning

1. **Concurrency Settings**:
   ```rust
   // Adjust based on node capacity
   let semaphore = Semaphore::new(100);
   ```

2. **Cache Sizes**:
   ```rust
   const MAX_SEEN_HASHES: usize = 10_000;
   const MAX_KNOWN_TXS: usize = 20_000;
   ```

3. **Network Timeouts**:
   - Set appropriate timeouts for RPC calls
   - Implement retry logic with backoff

## Monitoring & Alerting

### Key Metrics

1. **Transaction Processing**:
   - Transactions per second (TPS)
   - High-value transaction count
   - Processing latency

2. **Pool Monitoring**:
   - Number of tracked pools
   - Total value locked (TVL)
   - Rug pull detections

3. **System Health**:
   - Memory usage
   - CPU utilization
   - WebSocket connection status

### Logging

Use structured logging with appropriate levels:
```rust
info!("High-value transaction: {}", tx_hash);
warn!("Pool state is stale: {}", pool_address);
error!("Failed to process transaction: {}", e);
```

## Maintenance

### Regular Tasks

1. **Database Cleanup**: Archive old transaction data
2. **Log Rotation**: Implement log rotation policies
3. **Dependency Updates**: Keep dependencies current
   ```bash
   cargo update
   cargo audit
   ```

### Troubleshooting

Common issues and solutions:

1. **WebSocket Disconnections**:
   - Implement automatic reconnection
   - Check node WebSocket settings

2. **Memory Growth**:
   - Review cache eviction policies
   - Check for memory leaks with valgrind

3. **Performance Degradation**:
   - Analyze with timing tools
   - Review concurrent request limits

## Best Practices

1. **Always validate data** from external sources
2. **Never trust stale information** for critical decisions
3. **Monitor everything** - you can't improve what you don't measure
4. **Test with real data** - mock data hides real-world issues
5. **Plan for failure** - networks fail, nodes crash, data corrupts
6. **Keep it simple** - complexity is the enemy of reliability

## Support

For issues or questions:
- Check logs in `/home/nima/code/crypto/logs/`
- Review the audit report: `CRITICAL_AUDIT_REPORT.md`
- Consult the main documentation: `README.md`