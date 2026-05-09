# Solidity

Agent operating map for on-chain execution contracts used by the ETH stack.

## Purpose

- Host Solidity contracts that off-chain Rust planners/executors may target for
  swaps, settlement, guarded builder payments, and future adapters.
- Keep ABI/selector/bytecode changes discoverable for Rust calldata builders
  and transaction executors.

## Owns

- Baygus executor contract project under `baygus-executor/`.
- Solidity interfaces, libraries, command encodings, executor contracts, and
  Foundry tests for on-chain execution paths.
- Contract-level gas/safety work such as fast paths, permit handling,
  settlement accounting, custom errors, and reentrancy guards.

## Does Not Own

- Off-chain route discovery, simulation, or strategy policy.
- Rust calldata builders or transaction signing; use `reth_chain_query`,
  `tx_processor`, and `tx_executor`.
- Mempool/token analytics or alpha decisions.

## Data Flow

```text
tx_processor / mempool_processor / alpha decide a trade is valid
  -> reth_chain_query or planner builds calldata
  -> tx_simulator validates against local state
  -> tx_executor signs and broadcasts
  -> Baygus executor contract performs on-chain swaps/settlement
  -> receipts/traces return to Rust processing
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Contract project overview | `baygus-executor/README.md` |
| Deployment notes | `baygus-executor/DEPLOYMENT.md` |
| Foundry project | `baygus-executor/contracts/foundry.toml` |
| Solidity source map | `baygus-executor/contracts/src/README.md` |
| Interfaces | `baygus-executor/contracts/src/interfaces/` |
| Libraries | `baygus-executor/contracts/src/libraries/` |
| Tests | `baygus-executor/contracts/test/README.md` |
| Uniswap V4 contract notes | `baygus-executor/contracts/uniswap_v4/README.md` |

## Tests And Commands

```bash
cd solidity/baygus-executor/contracts
forge test
forge snapshot
```

## Current Hazards

- Rust builders/executors must be updated with any ABI, selector, or command
  encoding change.
- V2 single-hop gas parity is still a near-term focus; avoid adding generic
  dispatch cost to the common route without a measured reason.
- Multi-hop V3/V4 work needs flash/settlement accounting before it is treated
  as production-ready.
- Explicit `block.coinbase` payment behavior is contract calldata territory,
  not direct raw EOA transaction mode.
