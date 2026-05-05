# Baygus Executor

Baygus Executor is the Solidity execution layer used by the Ethereum simulation stack. Its job is to
turn a typed off-chain route into one on-chain transaction surface that can:

- execute Uniswap v4 `unlock -> swap -> settle/take` flows;
- net intermediate currencies across v4 paths;
- run simple command sequences such as pull, V2/Sushi/V3 swap, Curve swap, Balancer swap, flash
  loan, direct V2 pair swap, sweep, and bounded coinbase tips;
- keep protocol adapter addresses configurable at deployment.

## Design goal

Baygus should be cheap on-chain. The off-chain simulator and search pipeline should do every piece
of work that can be done before signing: route discovery, pool selection, calldata construction,
price checks, slippage bounds, block bounds, gas estimates, profit checks, and bribe sizing. The
deployed contract should only contain primitives that must execute atomically on-chain.

The full `BaygusExecutor` command surface is useful for validation and research, but it is not
automatically the cheapest production deployment. For live deployment, prefer the smallest executor
that covers the strategy's required hot path. Do not deploy Curve, Balancer, flash-loan, v4, hook,
or generic command support unless the strategy needs it and the gas benchmark justifies it.

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
- `0x09` `CMD_PERMIT2_TRANSFER_FROM`: pulls `msg.sender` tokens through Permit2 AllowanceTransfer.
- `0x0a` `CMD_TRANSFER_FROM`: pulls tokens from `msg.sender`.
- `0x0b` `CMD_COINBASE_TIP`: pays `block.coinbase` from executor native balance.
- `0x0c` `CMD_PERMIT2_SIGNATURE_TRANSFER_FROM`: pulls tokens through Permit2 SignatureTransfer.
- `0x0d` `CMD_V2_PAIR_SWAP`: transfers input directly to a Uniswap V2-compatible pair and calls
  `swap(amount0Out, amount1Out, recipient, "")`.

For hot one-hop V2/Sushi routes, prefer `CMD_V2_PAIR_SWAP` over the generic router adapter when the
off-chain planner already knows the pair and exact output. Its input is
`(pair, tokenIn, amountIn, amount0Out, amount1Out, recipient)`. Set `amountIn = 0` only when the
executor should spend its full current `tokenIn` balance. The command does not quote, discover
pairs, infer token order, or perform routing; those decisions belong in Rust/Python before signing.

Coinbase tips are intended for private bundles or carefully bounded public transactions. Use the
guarded input form `(amount, minBlock, maxBlock)` so a stale transaction cannot pay a builder in an
unexpected block. `msg.value` must fund the tip plus any native-input swap value.

ERC20 and Permit2 allowance-transfer inputs are only `(token, amount)` and always pull from
`msg.sender`. Explicit-owner allowance pulls are intentionally rejected so a public executor cannot
drain standing third-party allowances.

Permit2 signature-transfer input is
`(owner, token, permittedAmount, nonce, deadline, requestedAmount, signature)`. Use `owner =
address(0)` to default to `msg.sender`. The owner signs a Permit2 `PermitWitnessTransferFrom`
message for this executor as the spender. The witness is
`BaygusExecution(address executor,address caller,bytes32 commandsHash,bytes32 inputsHash)`, passed
to Permit2 with witness type string
`BaygusExecution witness)BaygusExecution(address executor,address caller,bytes32 commandsHash,bytes32 inputsHash)TokenPermissions(address token,uint256 amount)`.
It binds the signature to this executor, the transaction caller, command bytes, and all command
inputs except the signature bytes themselves. This avoids a prior Permit2 allowance, but the owner
must still have approved the ERC20 token to Permit2.

## Pre-deployment gate

Do not deploy a new executor until all of these are true:

- `forge fmt --check`, `forge build`, and `forge test` pass from `contracts/`.
- The Rust builders compile and their Baygus ABI tests pass.
- Constructor arguments are fixed: PoolManager and all adapter addresses are immutable after deploy.
- The bytecode hash and ABI diff are recorded next to the deployment note.
- The exact buy/sell/coinbase-tip plan is simulated against the target block state before signing.
- A gas benchmark proves the deployed surface is cheaper or strategically necessary versus direct
  router or pool calls. If not, move that logic off-chain or into a smaller executor.
