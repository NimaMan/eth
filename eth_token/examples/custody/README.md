# Custody and Pool-State Examples

Custody risk is a separate axis from pool routeability and reserve state.

Pool labels should answer these questions independently:

- `route:*`: can the current swap path buy or sell, and is the sell economic?
- `liquidity:*`: are AMM reserves present, dust, or actually removed?
- `custody:*`: does token logic have latent holder-control powers, or have those powers fired?
- `risk:*`: should a held position be considered terminal or unsafe?
- `lifecycle:*`: what broad pool lifecycle state does the pool state machine currently report?

This distinction matters for centralized assets such as USDT-style tokens: a
pool can have latent custody/freezer authority while still being liquid and
routeable. That is not the same thing as a rug. By contrast, an observed
event-less holder drain is realized custody risk and should make our held
position terminal even when the AMM reserves did not move.

Code should use `ERC20Token::current_pool_state_flags()` when it needs this
combined view: the pool contributes route/liquidity/risk state, and the token
contributes custody findings.

When state reconciliation finds a holder whose `balanceOf` is materially below
the Transfer-event ledger, call
`ERC20Token::record_reconciled_holder_confiscations(...)` to persist those
victims as realized custody findings. The method records only the missing
amount not already covered by trace-backed custody findings, so a traced vault
drain and later reconciliation of the same holder do not double-count.

## `pool_custody_flags`

Runs without Reth. It builds two in-memory pools:

- a Session-style event-less `transferFrom(vault, dead, amount)` holder drain;
- a USDT-style latent custody/freezer authority with healthy pool routeability.

```bash
cargo run -p eth_token --example pool_custody_flags
```

Expected shape:

- observed holder drain: `custody.realized=true`,
  `risk.holder_balance_backdoor_drain=true`,
  `risk.terminal_position_risk=true`, and
  `liquidity.reserve_liquidity_removed=false`;
- latent custody: `custody.latent=true`,
  `risk.terminal_position_risk=false`, and the buy/sell route stays effective.

## `custody_session_balance_drain`

Uses local Reth data to replay the real Session case and compare the
Transfer-event ledger against `balanceOf(vault)`.

```bash
cargo run -p eth_token --example custody_session_balance_drain
```

## `custody_session_scammer_case_report`

Builds the Risk Atlas case packet for the Session scammer analytics page. It
uses `reth_index/address_to_blocks` to replay token/pool active blocks, builds
buyer outcomes from token state reconciliation, joins pool-scoped token PnL, and
extracts buyer ETH fund-flow edges from `tx_fund_flow`. The default buyer
funding lookback is `50,000` blocks; override with
`SESSION_CASE_LOOKBACK_BLOCKS` for deeper lineage checks.

```bash
cargo run -p eth_token --example custody_session_scammer_case_report
```

Outputs are written under:

```text
risk_atlas/scammer_analytics/cases/eth_0x02467dd0_session_vault_balance_drain_25202411/
  artifacts/buyer_token_outcomes.{json,csv}
  artifacts/buyer_pnl.csv
  artifacts/tx_fund_flow/buyer_eth_edges.{json,csv}
  artifacts/tx_fund_flow/address_clusters.json
  reports/buyer_outcome_report.md
```
