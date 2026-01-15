#!/bin/bash

# Simple startup script for validation service
echo "Starting transaction validation service..."

# Set up paths
export PYTHONPATH="/home/nima/code/crypto/py/eth_data:$PYTHONPATH"
cd /home/nima/code/crypto/py/eth_data/scripts/provide_tx_service

# Use the conda python directly
/home/nima/miniconda3/envs/qw/bin/python validation_service.py