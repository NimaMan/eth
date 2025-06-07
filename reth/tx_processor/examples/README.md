# Examples

This directory contains usage examples for the REVM Transaction Simulator.

## Primary Examples

### `json_state_validator_no_rpc.rs` 
**Main transaction simulator** - Use this for most transaction simulation needs.

```bash
cargo run --example json_state_validator_no_rpc -- 0x<transaction_hash>
```

Features:
- ✅ Internal transfer tracking via CallTracer
- ✅ WETH-as-ETH handling
- ✅ Token symbol recognition (USDC, USDT, etc.)
- ✅ Proper decimal handling
- ✅ JSON output format

### `simulate_and_extract_diffs.rs`
Alternative implementation without internal transfer tracking.

```bash
cargo run --example simulate_and_extract_diffs -- 0x<transaction_hash>
```

## Specialized Examples

### `json_state_validator_with_manual_internals.rs`
Version with manual internal transfer extraction (for comparison/debugging).

### `mempool_like_alloy_db.rs`
Demonstrates mempool-style transaction simulation.

## Output Format

All examples output JSON in this format:

```json
{
  "0xaddress": {
    "eth_net": 1.23,
    "token_net": {
      "USDC": 1500.50,
      "WETH": -1.23
    },
    "movements": {
      "eth": {"in": {}, "out": {}},
      "token": {"in": {}, "out": {}}
    }
  }
}
```

## Testing

Use the validation scripts to test examples:

```bash
python3 ../scripts/validation/large_scale_validation.py --transactions 10
```