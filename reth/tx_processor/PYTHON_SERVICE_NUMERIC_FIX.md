# Python Validation Service Numeric Value Fix

## Issue
The Python validation service was returning numeric values (floats/integers) in JSON responses, but the Rust code expects string representations for these values to ensure precision and compatibility.

## Root Cause
The Python `ProcessedTxStateDiffCalculator` class generates state changes with numeric values:
- `eth_net`: float values for ETH balance changes
- `token_net`: float values for token balance changes
- `movements`: numeric values for individual transfer amounts

Additionally, event structures like ERC20 transfers and Uniswap swaps contained numeric amounts.

## Solution
Modified `/home/nima/code/crypto/py/eth_block_processor/scripts/provide_tx_service/validation_service.py` to convert all numeric values to strings before serialization.

### Changes Made:

1. **Added `convert_state_changes_for_rust()` function** (lines 112-152):
   - Converts `eth_net` values to strings
   - Converts `token_net` amount values to strings
   - Converts all movement amounts (both tokens and denom) to strings

2. **Updated event serialization**:
   - ERC20 transfers: `amount` → `str(transfer.amount)`
   - Internal transactions: `value` → `str(itxn.value)`
   - Uniswap V2 swaps: all amount fields converted to strings
   - Uniswap V4 swaps: amount, liquidity, and price fields converted to strings

3. **Applied the conversion** (line 235):
   - Changed from: `"state_changes": ptxn.state_changes if ptxn.state_changes else {}`
   - Changed to: `"state_changes": convert_state_changes_for_rust(ptxn.state_changes)`

## Testing
To test the changes:

1. Start the Python validation service:
   ```bash
   cd /home/nima/code/crypto/py/eth_block_processor/scripts/provide_tx_service
   python validation_service.py
   ```

2. Test with curl:
   ```bash
   curl -X POST http://127.0.0.1:18000/validate/transaction/0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae \
     -H "Content-Type: application/json" \
     -d '{"include_state_changes": true, "include_trace": true}' | jq .
   ```

3. Verify that all numeric values in the response are now strings.

## Impact
This change ensures compatibility between the Python validation service and Rust transaction processor, allowing for accurate comparison and validation of transaction processing results across both implementations.