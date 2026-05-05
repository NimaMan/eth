# Baygus Executor Deployment Gate

This executor is part of the transaction execution path. Treat every deploy as immutable production
infrastructure: the PoolManager and adapter addresses are constructor state and cannot be changed.

## Mainnet Constructor Inputs

- Uniswap v4 PoolManager: `0x000000000004444C5DC75cB358380d2E3de08a90`
- Uniswap v2 router: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
- SushiSwap router: `0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F`
- Uniswap v3 router: `0xE592427A0AEce92De3Edee1F18E0157C05861564`
- Balancer vault: `0xBA12222222228d8Ba445958a75a0704d566BF2C8`
- Permit2: `0x000000000022D473030F116dDEE9F6B43aC78BA3`

## Required Checks

Run from `soleth/baygus-executor/contracts`:

```bash
forge fmt --check
forge build
forge test
```

Run from `reth`:

```bash
cargo test -p tx_simulator tx_builders::baygus_executor --lib
cargo test -p tx_simulator default_artifact_paths_point_at_soleth --lib
cargo run -p tx_processor --example baygus_execution_plan
```

Before mainnet deployment, also run the exact deploy/buy/sell/coinbase-tip sequence through the
Reth-backed simulator at the intended block. The signed live transaction should use the same calldata
builder that passed simulation.

## Coinbase Tip Command

`CMD_COINBASE_TIP` pays `block.coinbase` from the executor native balance. The live executor should use
the guarded input form:

```solidity
abi.encode(amount, minBlock, maxBlock)
```

Set transaction `value` to include the tip plus any native-input swap amount. Use a tight `maxBlock`
for public mempool submission. For private bundles, target the bundle block and keep the same guard
so a delayed or replayed transaction cannot pay in an unintended block.

## Deployment Record

Record these values for every deployment:

- git commit
- chain id
- deployer
- deployed executor address
- constructor arguments
- `BaygusExecutor` bytecode hash
- artifact path used by Rust: `soleth/baygus-executor/out/BaygusExecutor.sol/BaygusExecutor.json`
- Foundry and Rust check output
