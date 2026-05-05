# Source

`BaygusRouter.sol` is the only production entry point.

It exposes two typed Uniswap v4 functions for Rust builders:

- `swapExactInputSingle(SwapExactInputSingleParams)`
- `swapExactInputPath(MultiHopParams)`

It also exposes `execute(bytes commands, bytes[] inputs)` for command sequences. Shared structs and
command ids live in `types/`; external protocol shapes live in `interfaces/`; transfer helpers live
in `libraries/`.

Execution commands are deliberately small and explicit. The router does not infer routes or look up
pools on-chain; off-chain Rust code must build the exact sequence, value, slippage bounds, and
coinbase-tip bounds before the transaction is signed.

For Uniswap v4, the mainnet PoolManager returns `BalanceDelta` as one packed `int256`, not as a
two-word Solidity struct. The router decodes that packed value before applying settlement. Positive
deltas are amounts to take from PoolManager; negative deltas are amounts to settle. ERC20
settlement uses the current v4 sequence: `sync(currency)`, transfer tokens to PoolManager, then
`settle()`.
