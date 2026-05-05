# Baygus Executor

Baygus Executor is the Solidity execution layer used by the Ethereum simulation stack. Its job is to
turn a typed off-chain route into one on-chain transaction surface that can:

- execute Uniswap v4 `unlock -> swap -> settle/take` flows;
- net intermediate currencies across v4 paths;
- run simple command sequences such as pull, V2/Sushi/V3 swap, Curve swap, Balancer swap, flash
  loan, sweep, and bounded coinbase tips;
- keep protocol adapter addresses configurable at deployment.

The Rust builders load Foundry artifacts from `out/`. Rebuild artifacts after contract changes:

```bash
cd contracts
forge build
forge test
```

The executor intentionally keeps calldata formats explicit. If a command needs a new behavior, add a
typed Solidity test first, then add the matching Rust builder.

## Current command surface

`execute(bytes commands, bytes[] inputs)` dispatches one byte per command:

- `0x01` `CMD_V4_SWAP`: forwards v4 unlock data to the configured PoolManager.
- `0x02` `CMD_V2_SWAP`: calls the configured Uniswap V2 router.
- `0x03` `CMD_V3_SWAP`: calls the configured Uniswap V3 router.
- `0x04` `CMD_SUSHISWAP`: calls the configured Sushi V2 router.
- `0x05` `CMD_CURVE_SWAP`: swaps through a provided Curve pool.
- `0x06` `CMD_BALANCER_SWAP`: swaps through the configured Balancer vault.
- `0x07` `CMD_SWEEP`: transfers the executor's token or native balance to a recipient.
- `0x08` `CMD_BALANCER_FLASH_LOAN`: starts a Balancer flash loan and executes a nested plan.
- `0x09` `CMD_PERMIT2_TRANSFER_FROM`: pulls tokens through Permit2 AllowanceTransfer.
- `0x0a` `CMD_TRANSFER_FROM`: pulls tokens from `msg.sender` or an explicit owner.
- `0x0b` `CMD_COINBASE_TIP`: pays `block.coinbase` from executor native balance.

Coinbase tips are intended for private bundles or carefully bounded public transactions. Use the
guarded input form `(amount, minBlock, maxBlock)` so a stale transaction cannot pay a builder in an
unexpected block. `msg.value` must fund the tip plus any native-input swap value.

Permit2 transfer input is either `(token, amount)` to pull from `msg.sender`, or
`(token, owner, amount)` to pull from an explicit owner. The owner must already have approved
Permit2 and granted this executor Permit2 allowance.

## Pre-deployment gate

Do not deploy a new executor until all of these are true:

- `forge fmt --check`, `forge build`, and `forge test` pass from `contracts/`.
- The Rust builders compile and their Baygus ABI tests pass.
- Constructor arguments are fixed: PoolManager and all adapter addresses are immutable after deploy.
- The bytecode hash and ABI diff are recorded next to the deployment note.
- The exact buy/sell/coinbase-tip plan is simulated against the target block state before signing.
