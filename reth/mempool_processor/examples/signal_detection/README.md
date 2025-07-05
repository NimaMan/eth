# Signal Detection Examples

This directory contains examples for testing and monitoring signal detection functionality, particularly for detecting liquidity removal transactions.

## Function Signature Detection

The signal detector identifies liquidity removal transactions by checking the first 4 bytes of the transaction input data (the function selector).

### Supported Function Signatures

- `0x02751cec` - removeLiquidityETH
- `0xbaa2abde` - removeLiquidity  
- `0xaf2979eb` - removeLiquidityETHSupportingFeeOnTransferTokens
- `0x5b0d5984` - removeLiquidityETHWithPermit
- `0xded9382a` - removeLiquidityETHWithPermitSupportingFeeOnTransferTokens

## Examples

### test_function_signature_detection.rs

Tests the function signature detection logic with known signatures and edge cases.

```bash
cargo run --example test_function_signature_detection --release
```

Features:
- Tests all known liquidity removal signatures
- Verifies non-liquidity functions are not detected
- Tests edge cases (empty data, short data, unknown signatures)
- Decodes actual transaction parameters

### monitor_liquidity_removals.rs

Monitors the mempool in real-time for liquidity removal transactions.

```bash
cargo run --example monitor_liquidity_removals --release
```

Features:
- Connects to mempool via IPC
- Detects liquidity removals in real-time
- Tracks statistics by function type and router
- Shows detection latency
- Provides periodic summaries

## How It Works

1. **Transaction Reception**: Receives pending transactions from mempool
2. **Input Data Parsing**: Extracts and decodes the input data field
3. **Signature Check**: Compares first 4 bytes against known signatures
4. **Router Identification**: Identifies which DEX router is being used
5. **Parameter Decoding**: For detected removals, decodes the parameters

## Example Output

```
🚨 LIQUIDITY REMOVAL DETECTED: 0x6994eb5361b3f627038cba6201d8e8ea31ab779d389b29b73cb5fc2299db1a1b removeLiquidityETH 0.0000 ETH via Uniswap V2 Router
   Token: 0xc30e4da69c8e91b8d9f156d850f597cbe674adb5
   Detection latency: 7μs
```

## Integration with Signal Detector

The `mempool_signal_detector` uses this detection to:
1. Filter transactions that might affect pool liquidity
2. Prioritize simulation of liquidity removal transactions
3. Generate alerts for significant liquidity drains