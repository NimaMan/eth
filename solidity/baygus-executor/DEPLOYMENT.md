# Baygus Executor Deployment Gate

This executor is part of the transaction execution path. Treat every deploy as immutable production
infrastructure: the PoolManager and adapter addresses are constructor state and cannot be changed.

## Deployment Principle

Deploy the least amount of code that can execute the strategy safely. Anything that can be computed,
selected, validated, or priced off-chain belongs in the Rust/Python pipeline, not in Solidity.

The on-chain executor should only provide atomic actions that cannot be safely split across
transactions, such as pulling funds with a bound signature, executing the selected swap path,
sweeping proceeds, and paying a bounded coinbase tip. Do not include generic adapters or command
families in a production deploy just because the research executor supports them.

Before mainnet deployment, classify each command as one of:

- required for the current live strategy;
- useful for simulation or research only;
- cheaper to replace with direct router/pool calldata.

Only the first category should be in the deployed production bytecode.

## Mainnet Constructor Inputs

- Uniswap v4 PoolManager: `0x000000000004444C5DC75cB358380d2E3de08a90`
- Uniswap v2 router: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
- SushiSwap router: `0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F`
- Uniswap v3 router: `0xE592427A0AEce92De3Edee1F18E0157C05861564`
- Balancer vault: `0xBA12222222228d8Ba445958a75a0704d566BF2C8`
- Permit2: `0x000000000022D473030F116dDEE9F6B43aC78BA3`

## Required Checks

Run from `solidity/baygus-executor/contracts`:

```bash
forge fmt --check
forge build
forge test
```

Run from `reth`:

```bash
cargo test -p tx_simulator tx_builders::baygus_executor --lib
cargo test -p tx_simulator default_artifact_paths_point_at_solidity --lib
cargo run -p tx_processor --example baygus_execution_plan
```

Before mainnet deployment, also run the exact deploy/buy/sell/coinbase-tip sequence through the
Reth-backed simulator at the intended block. The signed live transaction should use the same calldata
builder that passed simulation.

## Gas Gate

Every proposed deploy must include a gas comparison against the direct path it replaces. For a plain
one-hop swap, direct router or direct pair/pool execution is the baseline. Baygus is acceptable only
when its extra gas buys an execution property we need, such as atomic multi-step execution,
Permit2 witness binding, private-bundle bribe handling, or exact simulator-to-chain plan matching.

Latest local benchmark at block `25030123` for 1 WETH -> stablecoin routes:

- Generic Baygus router command, output sent directly to the buyer: about `+47k` to `+50k` gas
  over the direct router path.
- Baygus direct V2 pair command: about `+17k` to `+18k` gas over the direct router path for
  Uniswap V2/Sushi V2 WETH/stable routes.
- Permit2 AllowanceTransfer is still more expensive than plain ERC20 allowance for execution; use it
  only when the allowance model is worth the gas.

If a command adds overhead without being required by the live path, remove it from the production
executor and keep it in the research executor or off-chain planner.

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
- deployed runtime size
- deploy gas and hot-path execution gas versus direct execution
- artifact path used by Rust: `solidity/baygus-executor/out/BaygusExecutor.sol/BaygusExecutor.json`
- Foundry and Rust check output
