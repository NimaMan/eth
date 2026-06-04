# Top-100 Trader PnL Assessment

**Purpose.** Record the consolidated, on-chain-verified assessment of the top-100
addresses (by aggregated `total_pnl_denom`) in run
`token-pnl-semantic-traders-50k-20260602-01` (Uniswap-style token trading, blocks
25180546..=25230545, ~7 days). This is the Tier-One backtest-validity gate: it shows
that the current address-level PnL leaderboard is **not** a trader-edge signal, names
the calc/DB defects that inflate it, and states what must be fixed before address PnL
can gate a backtest. Labels and flags follow `trader_label_taxonomy.md`.

---

## (a) Headline findings

- **Zero `clean_spot_trader_eoa` in the top-100.** Not one address buys/sells on
  non-scam pools, pays its own gas, and has few pools. The entire top of the
  leaderboard is infrastructure, automated bots, scam creators, rug snipers, off-pool
  sellers, or accounting contradictions. The validation target class is empty exactly
  where you would most expect "good traders" to surface.
- **`infra_mislabeled_as_user` is a confirmed calc bug — 40 of 100.** Every
  router/bot/protocol contract in the cohort carries `is_user_candidate=true`; the
  infra/bot exclusion list is not applied. Confirmed on-chain on contracts including
  rank 1 (Banana-Gun-style router), rank 40 (CoW GPv2Settlement), rank 58 (Uniswap
  Universal Router, registry match), and rank 68 (Titan Builder). This single bug
  injects contracts directly into the trader leaderboard.
- **PnL is concentrated on scam pools — 91 of 100 carry
  `pnl_dominated_by_scam_pools` (>= 80% from `is_scam` pools).** Most "profit" is
  `liquidity_removed` rug / `custody_buyer_token_confiscation` cashflow. One
  confiscation rug, `0x12d44f8b`, is the dominant profit source for a large group of
  distinct addresses across the cohort (snipers, off-pool sellers, the Universal
  Router, CoW Settlement, custody anomalies).
- **`gas_not_attributed_to_address` is pervasive — 58 of 100.** These addresses pay 0
  gas; sampled `tx.from` is a different operator or many distinct end-user EOAs. Their
  PnL is a recipient leg or operator-paid contract flow, not one trader's economics.
- **`custody_victim_with_positive_pnl` contradictions — 8 of 100.** A
  `custody_victim_candidate` cannot net positive on a `can_buy=can_sell=false`
  honeypot. Two are genuine reconciliation calc bugs (rank 38: +22.42 ETH with 0
  persisted movement rows; rank 70: only inflow legs retained, buy-side outflow
  dropped -> impossible +4.87 ETH). The rest are mis-attributed roles (rank 29 seizer;
  rank 40 settlement-as-victim) or snipers that escaped pre-seize (ranks 91, 97, 98,
  100).
- **`movement_reconciliation_incomplete` — 62 of 100.** Worst cases are
  `aggregate_only` with `movement_rows_retained=0` (ranks 38, 86, 93, 97, 98): the PnL
  rests entirely on aggregate counters and is unauditable at the movement level.
- **65 of 100 provisional labels were corrected** during verification — chiefly
  snipers reclassified to off-pool sellers (no token legs at all), `scam_factory_contract`
  corrected to EIP-7702 `creator_scammer_eoa`, and `contract_unknown` corrected to
  `infra_router_or_bot`/`mev_arb_bot_contract`.

---

## (b) Label and verdict distribution

### Primary label distribution (n=100)

| Label | Count | Trust tier | PnL meaning |
| --- | ---: | --- | --- |
| `external_inflow_seller_eoa` | 29 | suspect / needs_review | Off-pool inflow / dump cashflow, not edge |
| `infra_router_or_bot` | 22 | exclude | Aggregated multi-user router flow |
| `mev_arb_bot_contract` | 17 | exclude / suspect | Automated bot flow |
| `creator_scammer_eoa` | 15 | suspect / exclude | Own-rug extraction |
| `fresh_launch_sniper_eoa` | 12 | needs_review / exclude | Scam-pool exit cashflow |
| `custody_anomaly_eoa` | 2 | suspect | Accounting contradiction |
| `infra_protocol` | 1 | exclude | Batched settlement flow |
| `contract_unknown` | 1 | suspect | Custom single-operator bot |
| `infra_mev_builder` | 1 | exclude | Block-reward aggregation |
| `clean_spot_trader_eoa` | **0** | likely_trustworthy | **none present** |

Entity split: 58 EOA, 33 contract, 9 known_infra. Infrastructure + automated-contract
labels total **42**; "trader-EOA"-bucket labels total **58** (none clean).

### Verdict distribution (n=100)

| Verdict | Count |
| --- | ---: |
| `exclude_not_a_trader` | 47 |
| `needs_review` | 27 |
| `suspect` | 26 |
| `likely_trustworthy` | **0** |

### Validity-flag prevalence (n=100)

| Flag | Count | Status |
| --- | ---: | --- |
| `pnl_dominated_by_scam_pools` | 91 | confirmed (>= 0.80 enforced; 7 over-flags removed) |
| `movement_reconciliation_incomplete` | 62 | confirmed where `retained < counted` |
| `gas_not_attributed_to_address` | 58 | confirmed where `tx.from != address` |
| `infra_mislabeled_as_user` | 40 | confirmed CALC BUG on all contracts |
| `custody_victim_with_positive_pnl` | 8 | 2 genuine calc bugs, 6 mis-attribution/escape |

### Summed PnL

PnL is monotonically descending by rank. The verifier rationales state explicit
per-address denom (ETH) totals only for part of the cohort, so summed-PnL is reported
qualitatively rather than fabricated:

- The two single largest rows are both **off-pool / aggregated, not traders**: rank 12
  (`external_inflow_seller_eoa`) at **164.18 ETH** (100% recipient/settlement inflow on
  others' rugs) and rank 21 (`external_inflow_seller_eoa`, recipient-only) at **98.76
  ETH** (207 legs all `native_denom_in`, zero token legs).
- Across the cohort, **0 ETH of the summed top-100 PnL is attributable to a
  `clean_spot_trader_eoa`**, since that class has zero members. The summed PnL of the
  `likely_trustworthy` verdict bucket is therefore **0 ETH**.
- The `creator_scammer_eoa` cluster (ranks 43-51, all EIP-7702 DeleGator EOAs) sits in
  a tight ~7.5 ETH band each (rank 43 7.77, 44 7.66, 45 7.53, 46 7.52, 47 7.52, 48
  7.52, 49 7.51, 50 7.51, 51 7.15) — a single operator's smart-EOA fleet running
  sequential rugs, all `suspect`.

---

## (c) Top ~30 addresses

PnL column = denom (ETH) where the verifier rationale states it explicitly; `n/s` =
not stated in the assessment (rows remain in descending-PnL rank order).

| Rank | Address | PnL (ETH) | Entity | Final label | Verdict | Key flags |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | `0x35fC556d6f8675b26FDf1542e6E894100155b34e` | n/s | known_infra | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, scam_dom, recon_incomplete |
| 2 | `0x71eA8223a24B82456F22716f787219bC15Db812B` | n/s | contract | mev_arb_bot_contract | suspect | infra_mislabeled, gas_not_attr, scam_dom, recon_incomplete |
| 3 | `0xfBB81382CCE6b9Ce58F8645353f70d9Db1BB69aF` | n/s | contract | mev_arb_bot_contract | suspect | infra_mislabeled, gas_not_attr, scam_dom, recon_incomplete |
| 4 | `0xf41c5683680f9DA33050577ec5C45a7d5f2F9be5` | n/s | contract | mev_arb_bot_contract | suspect | infra_mislabeled, gas_not_attr, recon_incomplete |
| 5 | `0x678B6d7e6D79AFbb1bEb45AFc384beA2084FF6d0` | n/s | contract | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, scam_dom (0.687), recon_incomplete |
| 6 | `0x356f4d1443516EDc5dF8606E06a799b2390Ca4d5` | n/s | EOA | external_inflow_seller_eoa | needs_review | gas_not_attr, scam_dom (100%) |
| 7 | `0x9ccF7d800cF38aC9a597a6bff2a083B01cccaa3A` | n/s | EOA | external_inflow_seller_eoa | needs_review | scam_dom (100%); self-paid gas |
| 8 | `0x7E7565b23ac04f4b4B4f9C6c2A00bf97b59cE473` | n/s | EOA | external_inflow_seller_eoa | needs_review | gas_not_attr, scam_dom (100%) |
| 9 | `0xDcBF277066E607446fE43Ad8275C7347d468a8B2` | n/s | contract | mev_arb_bot_contract | suspect | infra_mislabeled, gas_not_attr, scam_dom (0.878), recon_incomplete |
| 10 | `0x5703B683C7F928b721Ca95Da988d73a3299D4757` | n/s | contract | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, scam_dom (0.895), recon_incomplete |
| 11 | `0xc31a006c3e37e58eb1bb9ba752e023124f1a0f3f` | n/s | contract | mev_arb_bot_contract | exclude | infra_mislabeled, gas_not_attr, scam_dom (0.918); fully recon |
| 12 | `0xcf7f06192ad837653be626bf30ff0b85dccae385` | 164.18 | EOA | external_inflow_seller_eoa | suspect | scam_dom (100%), gas_not_attr |
| 13 | `0x79b0ad47888d8ca41a44a785222d68a20cca4a4c` | n/s | EOA | external_inflow_seller_eoa | needs_review | scam_dom (100%); self-paid gas |
| 14 | `0x50995f97f63f8ec9a131272269f1388d88a03990` | n/s | EOA | external_inflow_seller_eoa | suspect | gas_not_attr, scam_dom (100%) |
| 15 | `0x87543e2ee90ff3b6899814960f3bbcb6d7078b2d` | n/s | contract | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete (scam 73.2%, flag omitted) |
| 16 | `0xe84f01477986d36b12673e8dd98099dc440ee131` | n/s | EOA | external_inflow_seller_eoa | needs_review | scam_dom (100%); self-paid gas |
| 17 | `0x7ec117ed6e17a0e8eb643d9d3864081090b41b59` | n/s | contract | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.860) |
| 18 | `0xa250cc729bb3323e7933022a67b52200fe354767` | n/s | contract | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.930) |
| 19 | `0xf4bd77e26714134275abcedd0eb0eb2e81f72474` | n/s | EOA | external_inflow_seller_eoa | needs_review | scam_dom (100%); self-paid gas |
| 20 | `0x9de8276958ca6c47cdfe9d0d75dd0a4795a4a5a5` | n/s | contract | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.912) |
| 21 | `0x36c11106814ef31b14eeb42391c39c95917f6019` | 98.76 | EOA | external_inflow_seller_eoa | exclude | gas_not_attr, scam_dom (100%), recon_incomplete |
| 22 | `0x52b7b9e768900e2cc509ff0109b900660431b5d7` | n/s | known_infra | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.894) |
| 23 | `0x2c3774f48d711635e1d47e268526771fca365e1e` | 82.60 | EOA | external_inflow_seller_eoa | exclude | gas_not_attr, scam_dom (100%), recon_incomplete |
| 24 | `0xe739168b997faf07c3c3cad198a5394799fa2c1b` | n/s | contract | mev_arb_bot_contract | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.762) |
| 25 | `0xc7bbec68d12a0d1830360f8ec58fa599ba1b0e9b` | n/s | contract | mev_arb_bot_contract | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.934) |
| 26 | `0xd0b564200ae67a32a43256d37744ce554f49b7fd` | 48.90 | EOA | external_inflow_seller_eoa | needs_review | scam_dom (100%); self-paid gas |
| 27 | `0x440c1a9786d3c6f09da46b6d42320e0cf2f336a7` | 46.70 | known_infra | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom (0.863) |
| 28 | `0x5455c918e405a2831fbff8595c0aae35ee3db9d1` | 46.57 | known_infra | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete, scam_dom |
| 29 | `0x6bb6a6f25aa561a22586ff3e55f87ba41ca51a4c` | 45.00 | EOA | creator_scammer_eoa | suspect | scam_dom, custody_victim_with_positive_pnl (role mis-attribution) |
| 30 | `0xef22e6a8e7857c52614bf956d0a6527af73a4d97` | n/s | known_infra | infra_router_or_bot | exclude | infra_mislabeled, gas_not_attr, recon_incomplete (scam 0.473, flag omitted) |

Notable downstream rows: rank 40 `0x9008d19f58aabd9ed0d60971565aa8510560ab41`
(infra_protocol — CoW GPv2Settlement, +9.09 ETH); rank 58
`0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad` (infra_router_or_bot — Uniswap Universal
Router, registry match); rank 68 `0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97`
(infra_mev_builder — Titan Builder).

---

## (d) Calc / DB defects to fix

1. **Apply the infra/bot exclusion to `is_user_candidate`** (root cause of
   `infra_mislabeled_as_user`, 40/100). At PnL build, any address whose `eth_getCode`
   is a `>0`-byte contract, or that matches `address_book.rs` /
   `discover_validators.rs`, or whose bytecode contains a swap-callback selector
   (`fa461e33`/`23a69e75`/`10d1e85c`), must have `is_user_candidate=false`. This alone
   removes ranks 1, 5, 10, 11, 15, 17, 18, 20, 22, 24, 25, 27, 28, 30, 33-35, 37, 58,
   60-67, 81, 83, 84, 89, 92, 94, 99 from the user leaderboard.
2. **Resolve the custody victim/seizer sign and role attribution.** The
   `custody_victim_candidate` role is assigned by a buyer heuristic and is wrongly
   attached to (a) the pool creator/drainer (rank 29) and (b) batch-settlement
   contracts (rank 40). Fix the role assigner so the seizer/creator and settlement
   contracts are never tagged victim; the positive PnL there is correct extraction, not
   a victim loss.
3. **Complete movement reconciliation / fix the dropped buy-side leg** (root cause of
   the genuine `custody_victim_with_positive_pnl` calc bugs at ranks 38, 70, and of 62
   `movement_reconciliation_incomplete` rows). On `custody_buyer_token_confiscation`
   pools the buy-side outflow is being dropped, so realized PnL double-counts received
   ETH without the buy cost — producing impossible positive PnL for confiscation
   victims. Persist movement rows for `aggregate_only` rows (ranks 38, 86, 93, 97, 98
   have `movement_rows_retained=0`), or mark their PnL unauditable so it cannot enter a
   trust tier.
4. **Attribute / group gas across the EOA that pays for a contract's swaps** (root
   cause of `gas_not_attributed_to_address`, 58/100). A PnL row with `gas_paid==0` is
   either a recipient leg (route the cashflow back to the funding/initiating EOA) or an
   operator-paid contract (group the contract's PnL under its operator and exclude it
   from per-trader edge). Do not credit recipient-only `native_denom_in` legs as a
   trader's realized profit.
5. **Enforce the `pnl_dominated_by_scam_pools` threshold deterministically.** Compute
   the exact scam-pool PnL share and apply the hard `>= 0.80` cutoff (do not eyeball);
   verification removed it from ranks 31/34/54/56/71 (45.9-77.3%) and confirmed its
   correct omission at ranks 15/30. Surface scam-share as a stored column so the gate
   is reproducible.
6. **De-duplicate co-leg contracts.** Ranks 27/28 are two distinct contracts sharing
   431/432 tx_hashes inside the same bundled router transactions; their PnL is the same
   flow counted on two legs. Group bundled co-legs so one bundle is not double-credited.

---

## (e) What this means for using address PnL as a backtest-validity gate

- **Do not gate on the raw top-of-leaderboard.** In this 7-day window the top-100 by
  PnL contains **no** trustworthy trader; 47% are explicitly `exclude_not_a_trader` and
  the remaining 53% are `suspect`/`needs_review` rug/off-pool/contradiction rows. A
  backtest that treats "high PnL address" as ground-truth edge would be calibrating
  against routers, MEV bots, scam creators, and accounting artifacts.
- **The gate must be label-aware, not value-aware.** Only `clean_spot_trader_eoa` rows
  with no stacked validity flag are admissible. Until the `infra_mislabeled_as_user`
  exclusion (defect 1) and the gas-attribution grouping (defect 4) are fixed, even the
  EOA subset is contaminated by recipient legs and operator-paid flow.
- **Scam-pool cashflow is not edge.** With 91/100 dominated by `is_scam` pools (much of
  it the single rug `0x12d44f8b`), address PnL in this window primarily measures who
  exited a rug in time, who drained their own rug, and who collected proceeds off-pool
  — none of which generalizes to a tradeable strategy.
- **Unauditable rows cannot validate anything.** 62/100 have incomplete movement
  reconciliation and several rest on zero persisted movement rows; their PnL number
  cannot be reconstructed, so it must be excluded from any validity claim rather than
  trusted.
- **Path to a usable gate.** After defects 1-6 are fixed, re-run the taxonomy
  classifier (see `trader_label_taxonomy.md`, "How to apply later") across all 20,146
  addresses, then build the trader-edge cohort from `clean_spot_trader_eoa` /
  `likely_trustworthy` only. The presence (and PnL magnitude) of that cohort — not the
  raw leaderboard — is the metric the backtest-validity gate should consume.
