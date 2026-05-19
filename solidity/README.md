# Solidity

This directory is archival. The current Uniswap V4 simulation path does not
deploy or call repository-local Solidity executors.

Current production simulation flow:

```text
tx_processor pool buy/sell simulator
  -> tx_simulator Universal Router V4 calldata builder
  -> deployed Uniswap Universal Router + Permit2
  -> processed traces return to tx_processor / eth_token
```

Keep new pool-simulation work in `tx_simulator` and `tx_processor` unless a
measured production transaction path requires a new on-chain contract.

For contracts that are actually deployed on Ethereum, keep the operational
deployment runbook, chain-specific config, audit checklist, Kartal dry-run
evidence, and receipt records under `../onchain-deployments/`. This folder owns
source and tests, not the live deployment trail.
