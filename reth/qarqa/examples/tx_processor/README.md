# TX Processor Examples

This directory contains comprehensive examples demonstrating the `tx_processor` module functionality for Ethereum transaction simulation and fund flow analysis.

## Directory Structure

### `basic/` - Core Functionality
Examples showing fundamental transaction processing capabilities:
- **Development simulation** without REVM for testing
- **Fund flow analysis** with configurable parameters  
- **State change tracking** across transactions
- **REVM integration** for production-grade simulation

### `production/` - Real-World Use Cases
Production-ready examples for common analysis patterns:
- **Arbitrage detection** across DEX protocols
- **MEV analysis** for maximal extractable value
- **Whale tracking** for large holder movements
- **Token launch analysis** for new token deployments
- **Scam detection** using suspicious transaction patterns

### `integration/` - Module Integration
Examples showing integration with other QARQA modules:
- **Network building** from fund flow data
- **Database integration** for historical analysis
- **Live mempool processing** for real-time analysis
- **Portfolio tracking** for address monitoring

### `performance/` - Optimization & Benchmarks
Performance-focused examples and benchmarks:
- **Simulator benchmarks** comparing different implementations
- **Memory optimization** for large transaction batches
- **Batch processing** strategies for high throughput
- **Accuracy validation** against known results

### `testing/` - Error Handling & Edge Cases
Robust testing examples covering failure scenarios:
- **Error handling** for network failures and invalid data
- **Edge cases** for unusual transaction patterns
- **Resilience testing** under adverse conditions

## Running Examples

### Prerequisites
- Rust toolchain with Cargo
- PostgreSQL database (for integration examples)
- Local Ethereum node (optional, for RPC examples)

### Basic Examples
```bash
# Run development simulator
cargo run --example development_simulator

# Run fund flow analysis
cargo run --example fund_flow_analysis

# Run REVM integration
cargo run --example revm_integration
```

### Production Examples
```bash
# Detect arbitrage opportunities
cargo run --example arbitrage_detection -- --tx-hash 0x...

# Analyze MEV extraction
cargo run --example mev_analysis -- --block-number 18000000

# Track whale movements
cargo run --example whale_tracking -- --address 0x...
```

### Integration Examples
```bash
# Build network from transactions
cargo run --example network_building -- --tx-list transactions.txt

# Run with database
DATABASE_URL=postgresql://user:pass@localhost/qarqa cargo run --example database_integration
```

### Performance Examples
```bash
# Benchmark simulators
cargo run --example benchmark_simulators --release

# Test memory optimization
cargo run --example memory_optimization --release -- --batch-size 1000
```

## Example Data

Examples use real Ethereum transaction data where possible:
- **Real transaction hashes** from mainnet
- **Known arbitrage transactions** for validation
- **Complex DeFi interactions** for comprehensive testing
- **Scam transactions** for pattern recognition training

## Configuration

Many examples support configuration via:
- **Command line arguments** for transaction hashes, addresses, block ranges
- **Environment variables** for database URLs, RPC endpoints
- **Configuration files** for complex analysis parameters

## Documentation

Each example includes:
- **Comprehensive comments** explaining the analysis logic
- **Input/output documentation** for data formats
- **Performance notes** for optimization considerations
- **Use case descriptions** for practical applications

## Integration with QARQA

These examples demonstrate how `tx_processor` integrates with:
- **core_types** for shared data structures
- **data_access** for database operations
- **network_building** for graph construction
- **api_layer** for web service integration

## Production Readiness

All examples follow CLAUDE.md guidelines:
- **No mock data** - only real blockchain data
- **Production patterns** for error handling and resilience
- **Performance optimization** for high-throughput scenarios
- **Comprehensive documentation** for operational use

## Development Notes

When adding new examples:
1. Follow the existing naming convention
2. Include comprehensive documentation
3. Use real transaction data
4. Add performance considerations
5. Include error handling
6. Update this README