# Types

Shared command ids, Uniswap v4 structs, adapter config, and router errors.

The command ids are part of the Rust/Solidity contract. Update Rust builders and tests whenever a
command id or encoded input shape changes.

`CMD_COINBASE_TIP` accepts either `abi.encode(amount)` or
`abi.encode(amount, minBlock, maxBlock)`. Prefer the guarded form for live execution.
