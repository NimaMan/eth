#!/bin/bash
# MCP Server Starter - Simple wrapper for Claude Code

export PYTHONPATH="/home/nima/code/crypto/py:/home/nima/code/crypto/py/eth_data"
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/eth_db"
export PYTHONUNBUFFERED=1

exec /home/nima/miniconda3/envs/qw/bin/python /home/nima/code/crypto/py/eth_data/mcp_server/eth_data_mcp_server.py "$@"