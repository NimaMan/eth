# V2 Vault Simulations

This folder stores deployed-vault simulations for the mainnet
`UniswapV2TradingVault`.

The simulation gate complements the Solidity tests:

- Solidity unit and fork tests prove the contract behavior and gas envelope.
- These simulations prove the deployed mainnet vault can be exercised through
  the Rust tx simulator against local Reth state.
- Direct-router simulations provide a non-vault baseline for functional and
  gas comparison. They do not replace the vault gate because live execution
  uses the deployed vault target and ETH tx executor allowlist.
- ETH tx executor dry-runs prove the executor policy accepts only the intended
  target and selectors.

The live readiness rule is:

- final pre-submit simulation uses exact vault calldata;
- direct-router simulation is only a comparison baseline;
- ETH tx executor live policy allowlists the vault target, not arbitrary router
  calls.

We accept the vault overhead over direct router calls because the vault owns the
token-side mechanics: bought tokens stay in the vault, the emergency-sell call
approves only the exact token amount needed for the router sell, and ETH
proceeds go to treasury.

## Required Cases

| Case | Purpose | Expected Result |
| --- | --- | --- |
| `usdc-buy-then-sell` | Liquid V2 happy path through the deployed vault. | Buy succeeds and emergency sell succeeds. |
| `direct-usdc-buy-approve-then-sell` | Equivalent liquid V2 path without the vault. | Direct buy, exact approve, and direct sell succeed. |
| `otto-wallet-transfer-then-sell` | Sell path for a token already held by the wallet, transferred into the vault first. | Transfer succeeds and emergency sell succeeds. |
| `direct-otto-wallet-approve-then-sell` | Equivalent wallet-held token sell without the vault. | Exact approve and direct sell succeed. |
| `otto-current-state-sell-negative` | Guardrail when the vault has no current token position. | Emergency sell fails in simulation before any live submission. |

The OTTO same-block `buy-then-sell` path is intentionally not a required
success case because the token currently reverts on the sell leg with
`TransferHelper: TRANSFER_FROM_FAILED`. Keep it as token-behavior evidence, not
as a vault readiness gate.

## Performance Gate

The script checks conservative ceilings around the deployed-vault simulator
reports:

| Metric | Ceiling |
| --- | ---: |
| USDC buy gas | `220000` |
| USDC emergency sell gas | `230000` |
| Direct USDC buy gas | `180000` |
| Direct USDC approve gas | `80000` |
| Direct USDC sell gas | `180000` |
| OTTO wallet transfer gas | `130000` |
| OTTO emergency sell gas | `260000` |
| Direct OTTO approve gas | `80000` |
| Direct OTTO sell gas | `220000` |

These ceilings are intentionally above the pinned fork gas benchmark because
they include live token behavior and current-state simulation variance. If a
case exceeds the ceiling, review the trace and update the threshold only with
new evidence.

## Run

```bash
cd /home/nima/code/crypto/blockchains/eth
deploy/onchain/uniswap-v2-trading-vault/scripts/08_run_simulation_suite.sh
```

Reports are written under `reports/<run-id>/`.
