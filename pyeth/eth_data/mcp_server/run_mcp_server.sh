#!/bin/bash

# MCP Server Runner Script
# This script properly sets up the environment and runs the ETH Data MCP server

# Set environment variables
export PYTHONPATH="/home/nima/code/crypto/py:/home/nima/code/crypto/py/eth_data"
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/eth_db"
export ETH_RPC_URL="http://localhost:8545"
export LOG_LEVEL="INFO"

# Run the MCP server using the qw conda environment Python
exec /home/nima/miniconda3/envs/qw/bin/python /home/nima/code/crypto/py/eth_data/mcp_server/eth_data_mcp_server.py