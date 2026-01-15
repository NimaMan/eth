# Database Integration Examples

This directory contains examples demonstrating how ETH Kartal integrates with the live trading database for processing buy/sell signals and managing positions.

## Overview

The trading system uses a PostgreSQL database (`live_trading_db`) to coordinate between components:

1. **Python Strategy Engine** → Writes signals to `trade_signals` table
2. **ETH Kartal** → Reads signals, executes trades, updates `executions` table
3. **Position Manager** → Tracks positions in `live_positions` table

## Examples

### 1. `signal_generator.rs` - Create Trading Signals

Simulates the Python strategy engine by generating BUY and SELL signals in the database.

```bash
# Generate a BUY signal for 0.015 ETH
cargo run --example signal_generator -- --buy --eth-amount 0.015

# Generate a SELL signal for 50% of position
cargo run --example signal_generator -- --sell --percentage 50

# Custom token and slippage
cargo run --example signal_generator -- --buy --eth-amount 0.1 \
    --token 0x1f9840a85d5af5bf1d1762f925bdaddc4201f984 \
    --slippage 5.0 \
    --priority critical
```

**Signal Format:**
- BUY signals specify ETH amount to spend
- SELL signals specify percentage of position to sell
- Includes slippage tolerance, gas limits, and priority

### 2. `signal_processor.rs` - Process Database Signals

Shows how ETH Kartal polls the database for new signals and executes them.

```bash
# Start the signal processor (polls every 1 second)
cargo run --example signal_processor

# Set custom database URL
DATABASE_URL=postgresql://user:pass@localhost:5432/live_trading_db \
cargo run --example signal_processor
```

**Features:**
- Polls `trade_signals` table for PENDING signals
- Converts database signals to ETH Kartal alerts
- Handles BUY and SELL with proper amount calculations
- Updates signal status and creates execution records

### 3. `position_updater.rs` - Track Position Lifecycle

Monitors execution confirmations and updates position states.

```bash
# Start the position updater
cargo run --example position_updater
```

**Position States:**
1. INIT → Position created but not filled
2. BUY_SUBMITTED → Buy transaction sent
3. BUY_CONFIRMED → Buy transaction confirmed on-chain
4. SELL_SUBMITTED → Sell transaction sent
5. SELL_CONFIRMED → Position closed with P&L calculated

### 4. `full_integration_demo.rs` - Complete Trading Cycle

Demonstrates the entire flow from signal generation to position closure.

```bash
# Run the full demo
cargo run --example full_integration_demo
```

**Demo Flow:**
1. Generate BUY signal (0.015 ETH)
2. Process and execute BUY
3. Show position status
4. Generate SELL signal (50% of position)
5. Process and execute SELL
6. Show final P&L and metrics

## Database Schema

### Key Tables

**trade_signals**
```sql
- signal_id (UUID) - Unique signal identifier
- wallet_id - Trading wallet
- token_address - Token to trade
- pool_address - Uniswap pool
- action - BUY or SELL
- amount - Amount in wei (0 for percentage-based)
- signal_value - JSON with details (percentage, eth_amount)
- signal_data - JSON with execution params (slippage, priority)
- status - PENDING → SENT → CONFIRMED/FAILED
```

**executions**
```sql
- signal_id - Links to trade_signals
- tx_hash - Blockchain transaction hash
- block_number - Confirmation block
- gas_used - Actual gas consumed
- status - SUCCESS or FAILED
- execution_time_ms - Total execution latency
- error_message - Failure reason if any
```

**live_positions**
```sql
- wallet_id - Trading wallet
- pool_id - References eth_db.pools
- token_address - Token being traded
- status - Position state machine
- entry_time/price/tx_hash - Buy details
- exit_time/price/tx_hash - Sell details
- pnl_eth - Profit/loss in ETH
- roi_percent - Return on investment
```

## Signal Flow

### How Signals Work

1. **Signal Creation**
   - Strategy engine detects opportunity
   - Writes signal to database with status='PENDING'
   - Signal includes all execution parameters

2. **Signal Processing**
   - ETH Kartal polls for PENDING signals
   - Updates status to 'SENT'
   - Executes transaction
   - Updates status to 'CONFIRMED' or 'FAILED'

3. **Position Tracking**
   - BUY creates/updates position to BUY_CONFIRMED
   - SELL updates position to SELL_CONFIRMED
   - P&L calculated on position closure

### Signal Persistence

- Signals remain in database permanently for audit trail
- ETH Kartal processes each signal exactly once
- Failed signals can be retried by creating new signal
- No signal expiration - use deadline_seconds for time limits

## Configuration

### Environment Variables

```bash
# Database connection
DATABASE_URL=postgresql://postgres:password@localhost:5432/live_trading_db

# Ethereum RPC
ETH_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY

# Logging
RUST_LOG=eth_kartal=debug,signal_processor=info
```

### Database Setup

```bash
# Create database
psql -U postgres -c "CREATE DATABASE live_trading_db;"

# Run schema
psql -U postgres -d live_trading_db -f /path/to/create_tables.sql

# Initialize with test data
python initialize_db.py
```

## Integration with Mempool Processor

While these examples use database polling, the mempool processor uses ZMQ for real-time alerts:

- **Mempool → ETH Kartal**: ZMQ pub/sub on port 5559
- **Strategy → Database**: Write signals for execution
- **ETH Kartal → Database**: Update execution results

This hybrid approach provides:
- Real-time response for MEV opportunities (ZMQ)
- Reliable signal delivery for strategic trades (Database)
- Complete audit trail of all trading activity

## Testing

### Local Testing Setup

1. Start local Ethereum node:
```bash
anvil --fork-url https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
```

2. Create test keystore:
```bash
cargo run --bin keystore_manager -- create --path ./test_keystore.json
```

3. Fund test wallet:
```bash
cast send YOUR_WALLET_ADDRESS --value 1ether --private-key ANVIL_PRIVATE_KEY
```

4. Run examples with local RPC:
```bash
ETH_RPC_URL=http://localhost:8545 cargo run --example signal_processor
```

### Performance Expectations

- Signal polling: 1 second intervals
- Execution latency: < 200ms for high priority
- Database operations: < 10ms
- Total signal → execution: < 2 seconds

## Troubleshooting

### "No pending signals"
- Check database connection
- Verify signals exist with status='PENDING'
- Check wallet_id matches

### "Insufficient balance"
- Fund the wallet with ETH and tokens
- Check token decimals match
- Verify amount calculations

### "Transaction failed"
- Check slippage tolerance
- Verify pool has liquidity
- Check gas price limits

### "Database connection failed"
- Verify PostgreSQL is running
- Check DATABASE_URL format
- Ensure database exists

## Production Considerations

1. **Security**
   - Use encrypted keystore files
   - Rotate database credentials
   - Implement rate limiting

2. **Reliability**
   - Add connection retry logic
   - Implement signal timeout handling
   - Monitor execution success rates

3. **Performance**
   - Use connection pooling
   - Index frequently queried columns
   - Archive old execution records

4. **Monitoring**
   - Track signal processing latency
   - Alert on execution failures
   - Monitor position P&L