# Source

`BaygusExecutor.sol` is the only production entry point.

It exposes two typed Uniswap v4 functions for Rust builders:

- `swapExactInputSingle(SwapExactInputSingleParams)`
- `swapExactInputPath(MultiHopParams)`

It also exposes `execute(bytes commands, bytes[] inputs)` for command sequences. Shared structs and
command ids live in `types/`; external protocol shapes live in `interfaces/`; transfer helpers live
in `libraries/`.

Execution commands are deliberately small and explicit. The router does not infer routes or look up
pools on-chain; off-chain Rust code must build the exact sequence, value, slippage bounds, and
coinbase-tip bounds before the transaction is signed.

`CMD_V2_PAIR_SWAP` is the preferred V2/Sushi hot-path primitive when the planner has already chosen
the pair. It transfers `tokenIn` to the pair and calls `swap` with explicit `amount0Out` and
`amount1Out`; it does not approve a router or sweep output by default.

Do not treat this source tree as a mandate to deploy every supported adapter. The production
executor should be the smallest bytecode that covers the active strategy. Keep broad protocol
coverage here for simulation and regression testing, then split or omit unused commands for a cheap
live deployment.

For Uniswap v4, the mainnet PoolManager returns `BalanceDelta` as one packed `int256`, not as a
two-word Solidity struct. The router decodes that packed value before applying settlement. Positive
deltas are amounts to take from PoolManager; negative deltas are amounts to settle. ERC20
settlement uses the current v4 sequence: `sync(currency)`, transfer tokens to PoolManager, then
`settle()`.
