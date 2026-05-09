# Types

Shared command ids, Uniswap v4 structs, adapter config, and router errors.

The command ids are part of the Rust/Solidity contract. Update Rust builders and tests whenever a
command id or encoded input shape changes.

New command ids must clear a gas and necessity check before they belong in production bytecode. If a
planner can decide or encode something off-chain, keep it out of the contract. The live executor
should contain only the commands needed for the active strategy's atomic execution path.

`CMD_COINBASE_TIP` accepts either `abi.encode(amount)` or
`abi.encode(amount, minBlock, maxBlock)`. Prefer the guarded form for live execution.

`CMD_V2_PAIR_SWAP` accepts
`abi.encode(pair, tokenIn, amountIn, amount0Out, amount1Out, recipient)`. The planner is responsible
for token order and output math; the contract only moves `tokenIn` into the pair and executes the
precomputed swap.
