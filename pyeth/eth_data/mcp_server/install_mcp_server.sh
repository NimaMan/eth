#!/bin/bash

# ETH Data MCP Server Installation Script
# =================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG_DIR="$HOME/.claude/mcp_configs"
CLAUDE_CONFIG="$HOME/.claude.json"

echo "Installing ETH Data MCP Server..."
echo "==========================================================="

# Check if conda environment exists
if ! conda env list | grep -q "^qw "; then
    echo "Error: 'qw' conda environment not found!"
    echo "Please activate the qw environment first: conda activate qw"
    exit 1
fi

# Create MCP config directory if it doesn't exist
if [ ! -d "$MCP_CONFIG_DIR" ]; then
    echo "Creating MCP config directory..."
    mkdir -p "$MCP_CONFIG_DIR"
fi

# Copy MCP configuration
echo "Installing MCP configuration..."
cp "$MCP_CONFIG_DIR/eth_data.json" "$MCP_CONFIG_DIR/eth_data.json.bak" 2>/dev/null || true

# Test the server can be imported
echo "Testing server imports..."
python -c "
import sys
sys.path.insert(0, '/home/nima/code/crypto/py/eth_data')
try:
    from eth_data.tx_provider.processed_transaction_provider import RustProcessedTransactionProvider
    print('✓ Successfully imported RustProcessedTransactionProvider')
except ImportError as e:
    print(f'✗ Failed to import: {e}')
    sys.exit(1)
"

if [ $? -ne 0 ]; then
    echo "Error: Failed to import required modules"
    echo "Please ensure rs_tx_processor is installed:"
    echo "  cd /home/nima/code/crypto/rust/tx_processor && maturin develop --release"
    exit 1
fi

# Make server executable
chmod +x "$SCRIPT_DIR/eth_data_mcp_server.py"

echo ""
echo "Installation complete!"
echo ""
echo "To use the MCP server with Claude Code:"
echo "----------------------------------------"
echo "1. Add to Claude using: claude mcp add eth-data -- python $SCRIPT_DIR/eth_data_mcp_server.py"
echo ""
echo "2. Or manually add to your Claude configuration:"
echo "   Edit: $CLAUDE_CONFIG"
echo "   Add to mcpServers section:"
echo '   "eth-data": {'
echo '     "command": "/home/nima/miniconda3/envs/qw/bin/python",'
echo '     "args": ["'$SCRIPT_DIR'/eth_data_mcp_server.py"],'
echo '     "env": {'
echo '       "PYTHONPATH": "/home/nima/code/crypto/py:/home/nima/code/crypto/py/eth_data",'
echo '       "DATABASE_URL": "postgresql://postgres:postgres@localhost:5432/eth_db"'
echo '     }'
echo '   }'
echo ""
echo "3. Use with Claude Code:"
echo "   --allowedTools 'mcp__eth-data.*'"
echo ""
echo "Available tools:"
echo "  - get_processed_tx_from_hash: Get fully processed transaction with decoded events"
echo "  - get_processed_txs_for_address: Get all processed transactions for an address"
echo "  - process_block: Process entire block with Python block processor"
echo "  - trace_fund_flow: Trace fund flows through multiple hops"