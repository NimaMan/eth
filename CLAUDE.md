## Available MCP Tools (Always Enabled)

### ETH Data Tools (mcp__eth-data)
These MCP tools are ALWAYS AVAILABLE for investigating Ethereum transactions and blockchain data:

#### Available Tools:
1. **`mcp__eth-data__get_processed_tx_from_hash`**
   - Get fully processed transaction with decoded events, internal txs, and state changes
   - Usage: `mcp__eth-data__get_processed_tx_from_hash(tx_hash="0x...")`
   - Returns: Complete transaction details with events, transfers, and state changes

2. **`mcp__eth-data__get_processed_txs_for_address`**
   - Get all processed transactions for an address within a block range
   - Usage: `mcp__eth-data__get_processed_txs_for_address(address="0x...", start_block=20000000, end_block=20001000, limit=100)`
   - Returns: List of transactions with summaries

3. **`mcp__eth-data__process_block`**
   - Process entire block using Python block processor
   - Usage: `mcp__eth-data__process_block(block_number=20000000, save_to_db=false)`
   - Returns: All transactions, token transfers, DEX swaps, failed txs

4. **`mcp__eth-data__trace_fund_flow`**
   - Trace fund flow through multiple hops
   - Usage: `mcp__eth-data__trace_fund_flow(tx_hash="0x...", depth=3)`
   - Returns: Money movement paths and involved addresses

**Note**: These tools use the high-performance Rust transaction processor (91.5x faster than Python).
They are automatically available in every Claude session - no special flags needed.

## Developer Communication Insights

### Critical Feedback Logs
- **Development Instruction Clarity**: Emphasize direct, focused task instructions
- **Command Execution Expectation**: Strict adherence to initial task specifications
- **Performance Critique**: Prioritize task completion over auxiliary discussions

### Key Learnings
- Developers seek precise, no-nonsense implementation
- Avoid unnecessary context or feature expansion
- Focus on the exact requirement at hand
- Demonstrate understanding by direct, targeted action

### Critical Guidance Reminder
- **STICK TO WHAT I ASK YOU TO IMPLEMENT.  NEVER CHANGE APPRAOCH WITHOUT ASKING ME**