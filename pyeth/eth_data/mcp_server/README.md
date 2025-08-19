# ETH Data MCP Server

## Overview

The ETH Data MCP (Model Context Protocol) Server provides Claude Code with direct access to Ethereum blockchain data processing capabilities using the high-performance Rust transaction processor backend.

## Key Features

- **Transaction Investigation**: Get fully processed transactions with decoded events, internal transactions, state changes
- **Address Analysis**: Retrieve all transactions for addresses with filtering options  
- **Block Processing**: Process entire blocks using the Python block processor
- **Fund Flow Tracing**: Trace fund flows across multiple transaction hops
- **High Performance**: 91.5x performance improvement via Rust backend integration

## Available Tools

### 1. get_processed_tx_from_hash
- **Purpose**: Get comprehensive details for a specific transaction
- **Parameters**: `tx_hash` (required) - Transaction hash (0x...)
- **Returns**: Complete transaction details with decoded events, internal transactions, state changes

### 2. get_processed_txs_for_address  
- **Purpose**: Get all processed transactions for an address
- **Parameters**: `address` (required), `start_block`, `end_block`, `limit` (optional)
- **Returns**: List of transactions with summary statistics

### 3. process_block
- **Purpose**: Process an entire block and get all transactions
- **Parameters**: `block_number` (required), `save_to_db` (optional)  
- **Returns**: All block transactions with processing details and summaries

### 4. trace_fund_flow
- **Purpose**: Trace fund flow from a transaction through multiple hops
- **Parameters**: `tx_hash` (required), `depth` (optional, max: 5)
- **Returns**: Fund flow analysis across transactions with addresses and values

## Installation & Setup

### Prerequisites
1. Conda environment `qw` with required packages
2. Rust tx_processor built with maturin  
3. PostgreSQL with eth_db database
4. Ethereum node accessible via RPC

### Installation Steps

1. **Build dependencies:**
   ```bash
   cd /home/nima/code/crypto/rust/tx_processor
   maturin develop --release
   ```

2. **Test imports:**
   ```bash
   cd /home/nima/code/crypto/py/eth_data/mcp_server
   bash install_mcp_server.sh
   ```

3. **Configure Claude in `~/.claude.json`:**
   ```json
   {
     "mcpServers": {
       "eth-data": {
         "command": "/home/nima/miniconda3/envs/qw/bin/python",
         "args": ["/home/nima/code/crypto/py/eth_data/mcp_server/eth_data_mcp_server.py"],
         "env": {
           "PYTHONPATH": "/home/nima/code/crypto/py/eth_data",
           "DATABASE_URL": "postgresql://postgres:postgres@localhost:5432/eth_db",
           "ETH_RPC_URL": "http://localhost:8545"
         }
       }
     }
   }
   ```

4. **Set permissions in `~/.claude/settings.local.json`:**
   ```json
   {
     "permissions": {
       "allow": ["mcp__eth-data.*"]
     }
   }
   ```

## Critical Configuration Details

### Protocol Version
- **MUST use `2024-11-05`** - Claude Code requires this specific version
- Older versions like `0.1.0` will cause "Server's protocol version is not supported" errors

### Python Paths
- **PYTHONPATH**: Must be `/home/nima/code/crypto/py/eth_data` 
- **Script location**: `/home/nima/code/crypto/py/eth_data/mcp_server/eth_data_mcp_server.py`
- **Import strategy**: Uses absolute paths to work regardless of Claude's working directory

### Environment Variables
- `DATABASE_URL`: PostgreSQL connection for transaction data
- `ETH_RPC_URL`: Ethereum node for blockchain queries
- `PYTHONPATH`: Module import path (critical for proper imports)

## Troubleshooting

### "1 MCP server failed"
1. Check MCP logs: `/home/nima/.cache/claude-cli-nodejs/-home-nima-code-crypto/mcp-logs-eth-data/`
2. Test server manually: `python eth_data_mcp_server.py < /dev/null`
3. Verify imports: `bash install_mcp_server.sh`

### "Server's protocol version is not supported"
- Ensure `protocolVersion: "2024-11-05"` in initialization response
- This was the main issue that prevented connection

### Import errors
- Verify PYTHONPATH points to `/home/nima/code/crypto/py/eth_data`
- Ensure rs_tx_processor built: `cd rust/tx_processor && maturin develop --release`
- Check conda environment accessible: `/home/nima/miniconda3/envs/qw/bin/python --version`

### Database connection errors  
- Verify PostgreSQL running: `systemctl status postgresql`
- Check database exists: `psql -h localhost -U postgres -l | grep eth_db`
- Test connection: `psql postgresql://postgres:postgres@localhost:5432/eth_db`

## Testing

### Manual Tests
```bash
# Test MCP protocol communication
python test_mcp_client.py

# Test all tools with sample data
python test_mcp_server.py

# Debug server startup
timeout 3 python eth_data_mcp_server.py < /dev/null 2>&1
```

### Verification Commands
```bash
# Check Claude recognizes server
claude /mcp

# Test tool availability in Claude
# Just start typing: "What happened in transaction 0x..."
```

## Architecture Notes

- **MCP Protocol**: JSON-RPC 2.0 over stdio transport
- **Process Model**: Claude spawns server as subprocess, communicates via stdin/stdout
- **Performance**: ~1825 tx/sec processing via Rust backend
- **Memory**: ~200MB with caching enabled
- **Security**: Read-only access, no file system access beyond imports

## Files Overview

- `eth_data_mcp_server.py`: Main MCP server implementation
- `test_mcp_server.py`: Comprehensive functionality tests  
- `test_mcp_client.py`: MCP protocol communication test
- `debug_mcp_server.py`: Debugging version with detailed logs
- `install_mcp_server.sh`: Installation verification script
- `README.md`: This documentation