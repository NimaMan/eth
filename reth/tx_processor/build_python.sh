#!/bin/bash

# Build Python bindings for tx_processor
# 
# This script builds the Rust tx_processor with Python bindings
# and installs it as a Python module.

set -e

echo "Building tx_processor Python bindings..."
echo "========================================"

# Check if maturin is installed
if ! command -v maturin &> /dev/null; then
    echo "maturin not found. Installing..."
    pip install maturin
fi

# Build and install the Python module
echo "Building with maturin..."
maturin develop --features python --release

echo ""
echo "Build complete! Testing import..."
python3 -c "import tx_processor_py; print('✅ tx_processor_py imported successfully')"

echo ""
echo "To use in Python:"
echo "  from tx_processor_py import TxProcessor"
echo "  processor = TxProcessor('/home/nima/.local/share/reth/mainnet')"
echo "  ptx = processor.process_transaction('0x...')"

echo ""
echo "To use in fund flow analyzer:"
echo "  from tx_processor_wrapper import RustProcessedTransactionProvider"
echo "  provider = RustProcessedTransactionProvider()"