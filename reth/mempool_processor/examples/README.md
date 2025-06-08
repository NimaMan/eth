# Mempool Processor Examples

This directory contains example implementations and analysis tools for the mempool processor.

## Available Examples

### `mempool_coverage_analysis/`
**Purpose**: Analyze what percentage of mined Ethereum transactions pass through public mempool vs private channels.

**Key Features**:
- Real-time mempool monitoring via WebSocket
- 1-hour production analysis with 53.2% coverage rate
- Race condition handling and timing analysis
- Comprehensive reporting with JSON output

**Usage**: See `mempool_coverage_analysis/README.md`

### `historical_simulation.rs`
**Purpose**: Historical transaction simulation using REVM for state diff analysis.

**Key Features**:
- Simulate past transactions against historical state
- Extract state changes and token movements
- Performance analysis and validation

**Usage**:
```bash
cargo run --example historical_simulation
```

## System Requirements
- Local Reth node at `127.0.0.1:8545`
- Python 3.10+ with web3, pandas (for Python components)
- Conda environment `qw` activated (for analysis tools)