# Baygus Trading Vault Deployment Gate

This vault is part of the transaction execution path. Treat every deploy as
immutable production infrastructure: owner, treasury, WETH, and router addresses
are constructor state and cannot be changed.

## Deployment Principle

Deploy the least amount of code that can execute the strategy safely. Anything that can be computed,
selected, validated, or priced off-chain belongs in the Rust/Python pipeline, not in Solidity.

The on-chain vault should only provide atomic actions that cannot be safely split
across transactions. For Mode A that is approve+sell in one emergency exit.
Generic adapters, v4 routing, Permit2, flash loans, and coinbase-tip commands
are intentionally not part of this deployment surface.

## Mainnet Constructor Inputs

- owner: operator EOA or signer-controlled execution address
- treasury: ETH proceeds recipient
- WETH: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
- Uniswap v2 router: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`

## Required Checks

Run from `solidity/baygus-executor/contracts`:

```bash
forge fmt --check
forge build
forge test
```

Run from `reth`:

```bash
cargo test -p eth_live_trading
cargo test -p tx_simulator tx_builders::protocols::baygus_v2_vault --lib
cargo test -p eth_alpha_engine
```

Before mainnet deployment, also run the exact deploy, buy, and emergency sell
sequence through the Reth-backed simulator at the intended block. The signed live
transaction should use the same calldata builder that passed simulation.

## Gas Gate

Every proposed deploy must include a gas comparison against:

- direct buy through router to the EOA;
- direct sell with pre-existing allowance;
- vault Mode A emergency sell with approve+sell.
- a fee-on-transfer token path, locally and on a representative mainnet fork
  fixture.

Mode A is acceptable only if the saved buy-time approval gas is worth the extra
emergency-sell gas under the current scam-exit rank model.

## Deployment Record

Record these values for every deployment:

- git commit
- chain id
- deployer
- deployed vault address
- constructor arguments
- `BaygusTradingVault` bytecode hash
- deployed runtime size
- deploy gas, buy gas, and emergency-sell gas versus direct execution
- Rust tx-builder commit and selector test output
- Foundry and Rust check output
