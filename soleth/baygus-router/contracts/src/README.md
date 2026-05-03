# Source

`BaygusRouter.sol` is the only production entry point.

It exposes two typed Uniswap v4 functions for Rust builders:

- `swapExactInputSingle(SwapExactInputSingleParams)`
- `swapExactInputPath(MultiHopParams)`

It also exposes `execute(bytes commands, bytes[] inputs)` for command sequences. Shared structs and
command ids live in `types/`; external protocol shapes live in `interfaces/`; transfer helpers live
in `libraries/`.
