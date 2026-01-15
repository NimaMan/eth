#!/usr/bin/env python3
"""
MCP Server Wrapper - Ensures proper environment and execution
"""
import os
import sys
import subprocess

# Set up environment
env = os.environ.copy()
env['PYTHONPATH'] = '/home/nima/code/crypto/py:/home/nima/code/crypto/py/eth_data'
env['DATABASE_URL'] = 'postgresql://postgres:postgres@localhost:5432/eth_db'
env['ETH_RPC_URL'] = 'http://localhost:8545'
env['LOG_LEVEL'] = 'INFO'
env['PYTHONUNBUFFERED'] = '1'

# Path to the actual MCP server
server_script = '/home/nima/code/crypto/py/eth_data/mcp_server/eth_data_mcp_server.py'
python_exe = '/home/nima/miniconda3/envs/qw/bin/python'

# Run the server, passing stdin/stdout/stderr through
try:
    proc = subprocess.Popen(
        [python_exe, server_script],
        env=env,
        stdin=sys.stdin,
        stdout=sys.stdout,
        stderr=sys.stderr,
        bufsize=0  # Unbuffered
    )
    
    # Wait for the process
    sys.exit(proc.wait())
    
except KeyboardInterrupt:
    proc.terminate()
    sys.exit(0)
except Exception as e:
    print(f"Error running MCP server: {e}", file=sys.stderr)
    sys.exit(1)