# Uniswap V4 Trading Vault Assessment

This folder is the isolated assessment and future deployment path for a
candidate `UniswapV4TradingVault`.

It is intentionally separate from
`onchain-deployments/uniswap-v2-trading-vault/`. V4 has a different risk model:
Universal Router commands, Permit2 approvals, PoolManager settlement, hooks,
pool-key validation, and route encoding. V2 evidence must not be reused as V4
signoff.

## Current Status

Status: first candidate implementation, deployment blocked.

The Solidity source under `solidity/baygus-executor/contracts/src/v4/` now has a
first `UniswapV4TradingVault` candidate. `scripts/05_deploy.sh` still fails
closed until fork tests, gas review, route policy, and Kartal policy are
complete.

## Required Flow

1. Build route fixtures in `simulations/route-fixtures/`.
2. Add tx_simulator route rehearsals for Universal Router and vault paths.
3. Implement the Solidity vault under `contracts/src/v4/`. Done for candidate
   v1; pending review.
4. Add unit, fork, and gas tests under `contracts/test/v4/` and
   `contracts/test/bench/v4/`. Candidate v1 has local unit/gas tests and a
   pinned mainnet fork gas test; pending review and report capture.
5. Fill candidate configs under `config/`.
6. Run preflight, build/hash, fork rehearsal, calldata generation, and Kartal
   dry-run scripts.
7. Complete `audit/checklist.yaml` and resolve every finding.
8. Only then enable a real deploy script for a dated run folder.

## Bribe Policy

For the current direct-raw path, bribe means EIP-1559 priority fee. Direct
`block.coinbase` transfers and private bundle payments are not part of the V4
vault scaffold and require a separate execution protocol.
