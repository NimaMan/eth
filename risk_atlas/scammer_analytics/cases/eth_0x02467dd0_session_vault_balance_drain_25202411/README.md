# Session Vault Balance Drain Case

Actor-centric case for the `Session` token where an owner-controlled contract
drained the live trading vault balance without ordinary ERC-20 allowance
evidence.

## Scope

- suspect owner: `0x02467dd05200e5B3FBcdddbF43735aE4d0681a59`
- control contract: `0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4`
- token: `0xc3A640bD249381F8097f44C1b61C46172068cDff`
- pool: `0x77a43d235c261436f0ad4577ce575ff0991adc1a`
- victim vault: `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`
- live trade: `trd_mpr6c45t_8dzz_g`
- real run: `alpha11-univ2-lp30-pool-update-block-hold16-live-real-broadcast-20260529-151921026902336Z`

## Current Hypothesis

The token implements a holder-balance drain path. The suspect called the control
contract after our buy. That call read the vault balance, then invoked the token
with `transferFrom(vault, dead, amount)` even though vault allowance to the
control contract was zero. The token emitted an `Approval(0)` log but no normal
`Transfer` log for the burned amount, so pool-price-only backtests kept valuing
an inventory that no longer existed.

## Investigator Instructions

This case is an evidence-collection case for a suspected creator-controlled
holder balance drain. The goal is to build a factual packet that can inform law
enforcement, exchange compliance teams, or other lawful recovery/escalation
channels. Keep every conclusion separated into on-chain fact, tool-derived
finding, and inference, and attach the transaction hash or artifact path that
supports it.

The first priority is transaction fund flow:

1. Reconstruct where the suspect funds came from before token creation, pool
   creation, and the drain transaction.
2. Reconstruct where ETH, WETH, token inventory, LP tokens, and gas-funding
   balances went after the drain.
3. Follow direct transfers, internal calls, swaps, unwraps, bridge deposits,
   forwarding contracts, fresh wallets, and tagged service deposit addresses.
4. Identify any possible centralized exchange, hosted wallet, bridge, fiat
   gateway, or other compliance-controlled touchpoint. Record only evidence and
   confidence, not unsupported attribution.
5. Build a timeline that ties funding, deployment, trading enablement, our buy,
   the balance drain, manual exit, and post-drain movement together.

For every fund-flow edge, capture:

- source address, destination address, transaction hash, block, UTC timestamp,
  asset, amount, and whether the movement is direct, internal, swap-derived, or
  inferred from a trace;
- known labels from block explorers or internal analytics, with the source of
  the label;
- the next action required, such as trace deeper, check CEX deposit, check bridge
  withdrawal side, or mark as low-confidence dead end.

Investigate possible lawful actions:

- prepare a law-enforcement packet with the victim vault, loss evidence, suspect
  addresses, transaction hashes, screenshots/exports, and a concise narrative;
- if funds reach a likely CEX or hosted service, prepare a compliance contact
  packet asking the service to preserve records and review whether freezing or
  KYC preservation is possible under its process;
- check whether bridge operators, hosted RPC or wallet infrastructure, token
  launch venues, or explorer abuse/reporting channels can preserve metadata or
  add warnings;
- document whether civil counsel, local cybercrime reporting, or insurer/report
  channels are relevant for the operator;
- avoid retaliation, unauthorized access, threats, harassment, doxxing, or any
  action that could compromise the evidence chain.

Required outputs for the next agent:

- update `reports/evidence_packet.md` with a clean law-enforcement-ready
  narrative and evidence table;
- create or update a fund-flow report under `reports/` and machine-readable
  edges under `artifacts/tx_fund_flow/`;
- add confidence levels and open questions for every suspected service or wallet
  cluster;
- make the case visible in the Risk Atlas UI at
  `http://100.96.34.94:40020/eth/risk-atlas/scammer-analytics`, with a detail
  page for `eth_0x02467dd0_session_vault_balance_drain_25202411`.

## Timeline

| Block | Tx | Event |
| ---: | --- | --- |
| `25202408` | `0x41731b558d...f8035737` | LP approval warning observed at pool creation/trading enable block. |
| `25202409` | `0xbe89b2a69...5f870704` | Real live buy confirmed. Vault received `9,871,580.343970612` Session tokens. |
| `25202411` | `0xf0e8542a...c43d8ba` | Suspect called the control contract and drained the vault balance to `93` tokens. |
| `25202464` | `0x875a44f7...5f870062a` | Manual exit sold the remaining `93` tokens for `0.000000645289543916 ETH`. |

## Backtest Treatment

The live backtest result set
`live-alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-block-frame-20260529-152040017864493Z`
bought this same pool across the sweep and left the positions in
`buy_confirmed`. For `alpha11-univ2-lp30-pool-update-block-hold16`, trade
`trd_mpr6c4zc_8fxz_40` continued to mark value from pool reserves after the
drain:

- block `25202409`: current value `0.004873964726377231 ETH`
- block `25202425`: current value `0.020380123064957084 ETH`
- block `25202436`: current value `0.045356795283255376 ETH`

No post-entry `balance_drain` or equivalent scam risk event was recorded. This
case should be used as a fixture for detecting vault balance loss that is not
visible as a normal sell, liquidity removal, or ERC-20 `Transfer` log.

## Buyer Outcome Investigation

The current buyer packet was generated with:

```bash
cargo run -p eth_token --example custody_session_scammer_case_report
```

It replays the indexed token/pool active window from block `25202408` through
`25202575`, reconciles every pool-out buyer against on-chain `balanceOf`, joins
pool-scoped token PnL, and expands ETH fund-flow for buyer/suspect activity over
a 50,000-block pre-buy lookback.

Current result:

- pool-out buyers: `67`
- confiscated buyers: `5`
- partial-exit buyers still holding dust/residual inventory: `11`
- buyers still holding at the latest indexed token activity block: `51`
- direct creator/control buyer links: `1` (`buyer_is_suspect`)
- shared-funder clusters: `1` weak two-buyer cluster

The report intentionally separates facts from attribution. At the current
50,000-block window there is no direct on-chain creator/control funding link for
the remaining confiscated buyers or the still-holding buyers, except the suspect
address itself. Those rows remain follow-up leads, not proven controlled wallets.

Artifacts:

- `artifacts/buyer_token_outcomes.json`
- `artifacts/buyer_token_outcomes.csv`
- `artifacts/buyer_pnl.csv`
- `artifacts/tx_fund_flow/buyer_eth_edges.json`
- `artifacts/tx_fund_flow/buyer_eth_edges.csv`
- `artifacts/tx_fund_flow/address_clusters.json`
- `reports/buyer_outcome_report.md`

## Run

```bash
cargo run -p eth_risk_atlas -- scammer-case \
  risk_atlas/scammer_analytics/cases/eth_0x02467dd0_session_vault_balance_drain_25202411/case.toml
```
