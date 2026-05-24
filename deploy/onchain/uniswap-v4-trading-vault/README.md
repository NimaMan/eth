# Uniswap V4 Trading Vault Assessment

This folder is the isolated assessment and future deployment path for a
candidate `UniswapV4TradingVault`.

It is intentionally separate from
`deploy/onchain/uniswap-v2-trading-vault/`. V4 has a different risk model:
Universal Router commands, Permit2 approvals, PoolManager settlement, hooks,
pool-key validation, and route encoding. V2 evidence must not be reused as V4
signoff.

## Current Status

Status: candidate implementation, pinned benchmark evidence, deploy calldata,
and a Kartal policy-rejection dry-run exist; mainnet broadcast remains blocked.

The Solidity source under `solidity/baygus-executor/contracts/src/v4/` now has a
first `UniswapV4TradingVault` candidate. `scripts/05_deploy.sh` still fails
closed until explicit operator signoff. After deployment, Kartal must be
switched from the currently deployed V2 vault allowlist to the new V4 vault
target and rerun with `EXPECT=accepted`.

Latest pinned rehearsal evidence:

| Check | Result |
| --- | ---: |
| Fork block | 25131251 |
| Direct route with allowance setup | 382,953 gas |
| Candidate vault buy + sell route | 354,887 gas |
| Candidate vault route delta vs direct full route | -28,066 gas |
| Candidate vault deploy | 1,228,234 gas |
| Candidate artifact hash | `2144f1b8380a07e75f58c7b7edfb9ae428b70faa37884f0fdf35fbb0a952392c` |

Report:
`simulations/reports/eth-usdc-500-no-hook-candidate-vault-rehearsal.json`.

Latest gate run:
`runs/20260519-v4-gates-202032Z/`.

That run records constructor args, init code hash, deploy gas estimate,
nonce-bound predicted address, representative buy/sell calldata, and Kartal
dry-run rejection evidence against the current live V2 allowlist.

## Required Flow

1. Review `audit/assessment-criteria.md` and treat every criterion as a
   deployment gate.
2. Build route fixtures in `simulations/route-fixtures/`.
3. Add tx_simulator route rehearsals for Universal Router and vault paths.
4. Implement the Solidity vault under `contracts/src/v4/`. Done for candidate
   v1; pending final review.
5. Add unit, fork, and gas tests under `contracts/test/v4/` and
   `contracts/test/bench/v4/`. Candidate v1 has local unit/gas tests and a
   pinned mainnet fork gas test with report capture.
6. Fill candidate configs under `config/`.
7. Run preflight, build/hash, fork rehearsal, calldata generation, and Kartal
   dry-run scripts. Current pre-deploy run is complete.
8. Complete `audit/checklist.yaml` and resolve every finding.
9. Only then enable a real deploy script for a dated run folder.

## Bribe Policy

For the current direct-raw path, bribe means EIP-1559 priority fee. Direct
`block.coinbase` transfers and private bundle payments are not part of the V4
vault scaffold and require a separate execution protocol.
