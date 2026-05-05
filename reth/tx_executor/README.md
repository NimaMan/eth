# Tx Executor

`tx_executor` is the gas-first Ethereum transaction execution core that Kartal will use for real
submission.

The executor does not choose routes, discover pools, quote trades, or decide strategy. Those jobs
belong in the simulator/planner. This crate receives a prepared direct transaction, validates it,
assigns a nonce when needed, applies priority-fee bribe policy, signs, broadcasts, and records the
result.

## Default Execution Rule

Use the cheapest direct transaction that satisfies the strategy:

- direct Uniswap V2/Sushi/V3 router calldata for normal one-hop swaps;
- direct raw calldata from the planner for custom routes;
- Baygus or another on-chain executor only when atomic multi-step behavior, guarded coinbase tips,
  Permit2 witness binding, v4 unlock flows, or other contract-only behavior is required.

For normal direct EOA transactions, the validator/builder bribe is the priority fee. Explicit
`block.coinbase` payments require contract calldata and are not part of direct raw mode.

## Current Surface

- `DirectRawTransactionRequest`: prepared EIP-1559 transaction request.
- `EthTxExecutor::submit_direct_raw`: validate -> reserve nonce -> sign -> broadcast.
- `MempoolPositionEstimator`: estimates gas before this transaction in the pending block by reading
  local Reth `txpool_content` and comparing effective priority fee.
- `JsonlRecorder`: optional append-only event journal for standalone runs.

## Example

Dry run only:

```bash
cargo run --example submit_direct_raw -- \
  --rpc-url http://127.0.0.1:8545 \
  --private-key-env ETH_EXECUTOR_PRIVATE_KEY \
  --to 0x0000000000000000000000000000000000000000 \
  --dry-run
```

Live broadcast is intentionally explicit:

```bash
cargo run --example submit_direct_raw -- \
  --rpc-url http://127.0.0.1:8545 \
  --private-key-env ETH_EXECUTOR_PRIVATE_KEY \
  --to 0x0000000000000000000000000000000000000000 \
  --broadcast
```

## Integration Direction

Kartal should wrap this library with HTTP routes and Postgres persistence:

- `POST /eth/tx/submit`
- `GET /eth/tx/submissions`
- `GET /eth/tx/submissions/{attempt_id}`

The submitted payload should include the simulator block/hash and expected/min output metadata so a
real transaction can be traced back to the exact off-chain decision that produced it.
