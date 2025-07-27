# Tax Calculation Examples

This directory contains an example program for analyzing token buy/sell taxes in Uniswap V2 pools.

## Example

- `analyze_tax_for_block_range.rs` - Generic tax analyzer for any token across a block range

## Usage

```bash
# Generic analyzer for any token
cargo run --example analyze_tax_for_block_range <token_address> <pool_address> <start_block> [end_block]

# Example usage:
cargo run --example analyze_tax_for_block_range 0x354ee0074cd5538a1dd1fda7a13e517e11b02699 0xa1eb81db04d93b210ba934a31d794eea76007b20 23002159 23002359
```

## How It Works

1. Fetches token info (name, symbol, decimals, total supply) from the blockchain
2. Simulates a 0.1 ETH buy transaction through Uniswap V2 router
3. Simulates selling 1% of the received tokens back
4. Calculates tax rates from the differences in expected vs actual amounts
5. Logs all results to `/home/nima/code/crypto/logs/test/`

## Output Format

The tool creates a log file with:
- Token details (name, symbol, decimals, total supply)
- Block-by-block analysis showing:
  - Buy simulation results
  - Sell simulation results
  - Calculated tax percentages
  - Any errors or restrictions