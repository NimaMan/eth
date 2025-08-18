#!/bin/bash

# Service wrapper script for systemd
# This script properly activates conda and starts the validation service

set -e

# Log startup
echo "🚀 Starting Transaction Validation Service..."
echo "Working directory: $(pwd)"
echo "User: $(whoami)"
echo "Date: $(date)"

# Source conda
if [ -f "/home/nima/miniconda3/etc/profile.d/conda.sh" ]; then
    echo "📦 Sourcing conda..."
    source /home/nima/miniconda3/etc/profile.d/conda.sh
else
    echo "❌ Conda not found at /home/nima/miniconda3/etc/profile.d/conda.sh"
    exit 1
fi

# Activate environment
echo "🔄 Activating conda environment 'qw'..."
conda activate qw

# Verify Python and dependencies
echo "🐍 Python version: $(python --version)"
echo "📍 Python path: $(which python)"

# Check critical dependencies
echo "🔍 Checking dependencies..."
python -c "import web3; print('✅ web3 OK')" || { echo "❌ web3 import failed"; exit 1; }
python -c "import fastapi; print('✅ fastapi OK')" || { echo "❌ fastapi import failed"; exit 1; }
python -c "import uvicorn; print('✅ uvicorn OK')" || { echo "❌ uvicorn import failed"; exit 1; }

# Check Reth connection
echo "🔗 Checking Reth node connection..."
if curl -s -X POST \
   -H "Content-Type: application/json" \
   -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
   http://127.0.0.1:8545 > /dev/null; then
    echo "✅ Reth node is running"
else
    echo "❌ Error: Cannot connect to Reth node at http://127.0.0.1:8545"
    echo "   Make sure your Reth node is running and accessible"
    exit 1
fi

# Add project to Python path
export PYTHONPATH="/home/nima/code/crypto/py/eth_data:$PYTHONPATH"

# Start the service
echo "🌐 Starting validation service on port 18000..."
exec python validation_service.py