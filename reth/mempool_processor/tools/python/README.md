# Python Analysis Tools

## Overview

Python utilities for analyzing and validating the Rust mempool processor system.

## Directory Structure

```
python/
├── core/                    # Core analysis and validation tools
│   ├── validate_state_changes.py          # Single transaction validation
│   ├── batch_validate_state_changes.py    # Batch transaction validation  
│   ├── analyze_mempool_tx_processor_performance.py  # Performance analysis
│   ├── consolidated_timing_analyzer.py    # Comprehensive timing analysis
│   ├── test_scam_detection.py            # Scam detection testing
│   └── run_timing_analysis.sh            # Analysis runner script
├── utils/                   # Utility tools
│   └── subscriber.py       # ZeroMQ message subscriber for debugging
└── README.md               # This file
```

## Quick Start

### **Performance Analysis** (Most Common)
```bash
cd core/
./run_timing_analysis.sh --no-plots
```

### **State Validation** (Development/Testing)
```bash
cd core/
python validate_state_changes.py --tx-hash 0x123...
```

### **Batch Validation** (Comprehensive Testing)
```bash
cd core/  
python batch_validate_state_changes.py --count 100
```

### **Scam Detection Testing**
```bash
cd core/
python test_scam_detection.py
```

## Tools Description

### Core Analysis Tools

- **`validate_state_changes.py`** - Validates single transaction state changes against REVM simulation
- **`batch_validate_state_changes.py`** - Runs validation across multiple transactions for comprehensive testing
- **`analyze_mempool_tx_processor_performance.py`** - Analyzes mempool processor performance metrics
- **`consolidated_timing_analyzer.py`** - Comprehensive timing analysis treating system as queuing model
- **`test_scam_detection.py`** - Tests scam detection algorithms and thresholds
- **`run_timing_analysis.sh`** - Easy-to-use script runner for timing analysis

### Utilities

- **`subscriber.py`** - ZeroMQ message subscriber for debugging and monitoring system messages

## Requirements

- Python 3.10+
- web3.py for Ethereum interactions
- Local reth node running on 127.0.0.1:8545
- Access to PostgreSQL database for some tools

## Environment

Tools expect to run in the `qw` conda environment with all dependencies installed.