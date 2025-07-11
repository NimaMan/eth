# Basic TX Processor Examples

This directory contains fundamental examples demonstrating core transaction processing capabilities.

## Examples Overview

### `development_simulator.rs`
Lightweight transaction simulator for testing and development without requiring REVM.

**Features:**
- Mock transaction execution for development
- Fund flow extraction without full state simulation
- Batch processing capabilities
- Performance testing baseline

**Use Cases:**
- Unit testing transaction logic
- Development environment setup
- Basic fund flow analysis
- Transaction pattern validation

### `fund_flow_analysis.rs`
Comprehensive fund flow analysis with configurable parameters.

**Features:**
- Configurable value thresholds
- Gas payment inclusion/exclusion
- WETH-to-ETH conversion
- Net balance calculation
- Flow aggregation between addresses

**Use Cases:**
- Trading pattern analysis
- Liquidity flow tracking
- Address relationship mapping
- Transaction categorization

### `state_changes.rs`
State change tracking and analysis across transaction execution.

**Features:**
- Balance change tracking
- USD value conversion
- Significance filtering
- Top movers identification
- Conservation verification

**Use Cases:**
- Portfolio impact analysis
- Wealth distribution tracking
- Transaction effect measurement
- Audit trail generation

### `revm_integration.rs`
Production-grade REVM integration for accurate transaction simulation.

**Features:**
- Full transaction execution simulation
- Internal transfer extraction
- Gas analysis and optimization
- State diff generation
- Execution trace processing

**Use Cases:**
- Production transaction analysis
- MEV detection and analysis
- Smart contract interaction analysis
- Accurate fund flow extraction

## Running Basic Examples

### Development Simulator
```bash
# Basic usage with synthetic transactions
cargo run --example development_simulator

# With custom transaction data
cargo run --example development_simulator -- --tx-file custom_txs.json

# Batch processing mode
cargo run --example development_simulator -- --batch-size 100
```

### Fund Flow Analysis
```bash
# Standard analysis
cargo run --example fund_flow_analysis

# With custom thresholds
cargo run --example fund_flow_analysis -- --min-value 0.1 --include-gas false

# WETH analysis mode
cargo run --example fund_flow_analysis -- --weth-as-eth true
```

### State Changes
```bash
# Basic state tracking
cargo run --example state_changes

# With USD conversion
cargo run --example state_changes -- --include-usd true

# Top movers analysis
cargo run --example state_changes -- --top-count 10
```

### REVM Integration
```bash
# Full REVM simulation
cargo run --example revm_integration

# With specific transaction
cargo run --example revm_integration -- --tx-hash 0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006

# Performance comparison mode
cargo run --example revm_integration -- --compare-simulators true
```

## Example Data

All examples use realistic transaction patterns:
- **Simple ETH transfers** for basic functionality
- **ERC-20 token swaps** for DeFi interactions
- **Complex DeFi transactions** for advanced analysis
- **Failed transactions** for error handling

## Configuration Options

Most examples support configuration via command line:
- `--min-value`: Minimum value threshold for analysis
- `--include-gas`: Include gas payments in analysis
- `--weth-as-eth`: Treat WETH as ETH for calculations
- `--batch-size`: Number of transactions to process together
- `--output-format`: JSON, CSV, or console output

## Performance Notes

- **Development simulator**: ~1000x faster than REVM but less accurate
- **REVM integration**: Production accuracy with ~100ms per transaction
- **Batch processing**: Significant speedup for multiple transactions
- **Memory usage**: ~10MB per 1000 transactions analyzed

## Integration Points

These basic examples demonstrate integration with:
- **core_types**: Using shared transaction and address types
- **data_access**: Fetching transaction data from database
- **network_building**: Preparing fund flows for graph construction

## Next Steps

After understanding basic functionality, explore:
- **Production examples** for real-world use cases
- **Integration examples** for module composition
- **Performance examples** for optimization techniques