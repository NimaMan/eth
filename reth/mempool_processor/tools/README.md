# Tools Directory

This directory contains analysis tools, utilities, and validation scripts for the mempool processor.

## Directory Structure

### `performance/`
**Purpose**: Performance testing and monitoring tools

**Tools**:
- `mempool_performance_analyzer.rs` - Analyze mempool processing performance
- `mempool_performance_analyzer_nodrain.rs` - Performance analysis without queue draining
- `revm_performance_monitor.rs` - Monitor REVM simulation performance
- `simple_performance_test.rs` - Basic performance testing

**Usage**:
```bash
cargo run --bin mempool_performance_analyzer
```

### `validation/`
**Purpose**: Validation scripts and testing utilities

**Tools**:
- `large_scale_validation.rs` - Large-scale transaction validation
- `run_validation_tests.rs` - Run comprehensive validation test suite
- `validate_tx/` - Individual transaction validation scripts (90+ files)

**Usage**:
```bash
cargo run --bin large_scale_validation
```

### `python/`
**Purpose**: Python analysis tools and utilities

**Key Components**:
- `core/` - Core analysis tools including timing analysis and batch validation
- `utils/` - Utility functions and subscriber implementations

**Features**:
- Consolidated timing analyzer for mempool residence times
- Batch validation of state changes
- EVM mining data collection
- Scam detection analysis

**Usage**:
```bash
cd tools/python
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw
python core/consolidated_timing_analyzer.py
```

## System Requirements
- Rust 1.86+ for Rust tools
- Python 3.10+ with conda environment `qw` for Python tools
- Local Reth node at `127.0.0.1:8545`
- PostgreSQL access for database validation tools