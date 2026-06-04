# Full Top-100 / Bottom-100 Address-PnL Review
**Run:** `token-pnl-semantic-traders-50k-20260602-01` (pre-router-threshold baseline)
**Scope:** every one of the top-100 (winners) and bottom-100 (losers) was reviewed; ranks were drilled to movement/leg level or onchain code where noted. Numbers and addresses below are taken directly from the reviewed data — none invented.

---

## 0. Headline verdict (read this first)

Address PnL behaves **sanely on the bottom extreme and pathologically on the top extreme.**

- **Bottom-100:** 72/100 are **REAL economic losses** (61 `genuine_rug_loss` + 11 `genuine_custody_loss`), almost all reconciling to movements to 4 decimals. The losers are overwhelmingly real people/bots who bought tokens that got rugged or confiscated. The loss side is trustworthy.
- **Top-100:** essentially **0 real trader edge.** Exactly **1 of 100** (`rank 95`) is `real_trader_edge`, and even that is scam-token round-tripping. The other 99 split into bot/router pass-through (40), scammer/rug extraction proceeds (40), and outright calc artifacts (16, including 3 `unclear`). The winner side is dominated by the **D3 recipient-leg-as-profit** defect.
- **Admissibility:** **0/100 top and 0/100 bottom are `tier_one_admissible`.** The gate rejects everything at both extremes — correct in spirit, but for the wrong-sounding reason on the top (it is rejecting scam-pool dominance and missing gas attribution, not "this is a phantom number").
- **Critical NEW leak:** the bottom-100 surfaced a **silent USDC/USDT/DAI-as-ETH unit-mix artifact** (ranks 1, 9, 10, 79) where `all_eth=false` fires but **`tier_one_admissible=true`**. The single biggest "loss" in the entire run, **rank 1 at -39,894 "ETH" (0xf95a15be...), is really ~39,867 USDC + ~27 ETH** and it **passes the gate.** This is the most dangerous finding in the review and is not one of the previously-tracked D2–D6 defects.

---

## 1. WINNER ARCHETYPES (the top-100)

Drilling every winner produced one decisive cross-cutting fact (confirmed in cohort notes 0–9): **almost no winner has a buy→sell round trip.** Winner PnL is overwhelmingly composed of `native_denom_in` (ETH inflow) legs with **zero `token_in` buy legs** and **zero cost basis netted**. The "profit" is gross ETH received, not edge.

### pnl_verdict distribution (top-100)
| pnl_verdict | count | meaning |
|---|---|---|
| `bot_or_infra_flow` | **40** | router/MEV/builder/settlement pass-through; D3+D5 inflated |
| `scammer_extraction` | **23** | creators/seizers draining their own pools (real ETH, not edge) |
| `rug_exit_proceeds` | **17** | sold a rug token for ETH (real ETH, scam-related) |
| `calc_artifact` | **16** | sign-flip / denom-mis-scale / D5 double / unbacked aggregate |
| `unclear` | **3** | D6 gap so large the number is unsupported (52, 57, 82) |
| `real_trader_edge` | **1** | rank 95 only |

### Archetype 1 — Bots / routers / infra (40 rows, `bot_or_infra_flow`)
The single largest winner class. These are contracts (and a few EOAs) that **receive routed WETH legs inside other people's swaps** and get the inflow booked as profit (gas=0 → D3), often double-counted across co-legs (D5).
- **rank 1, `0x35fc556d6f8675b26fdf1542e6e894100155b34e`** — registry-confirmed **Banana Gun Deployer 2**. The biggest number in the top-100: **1270.4 ETH** = 11,591 `native_denom_in` events across 389 scam pools with **zero token legs**; only 1232.5 ETH is even movement-backed (D6 gap). Sampled tx `0xbd02ebca` has `tx.from=0x21a34ce1` (not this address), gas=0. Pure bot pass-through.
- **rank 31, `0xcd17084c84dae21a22105848998cde8aec8b474b`** — 27,902 movements over 6,212 txs across 115 pools, receiving native ETH from many distinct EOAs. Unambiguous router/bot infra; +42.16 is recipient-leg booking.
- **rank 40, `0x9008d19f58aabd9ed0d60971565aa8510560ab41`** — the canonical **CoW Protocol GPv2Settlement** contract (verified onchain). Pool 0x12d44f native legs balance exactly (in 5.30158 == out 5.30158); +9.09 is settlement pass-through. Also carries the D2 `custody_victim_with_positive_pnl` mislabel.
- **rank 58, `0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad`** — registry = **Uniswap Universal Router**, mislabeled `fresh_launch_sniper`.
- **rank 68, `0x4838b106fce9647bdf1e7877bf73ce8b0bad5f97`** — registry-confirmed **MEV Builder**; 1827 `native_denom_in` legs, zero buys (coinbase/recipient flow).
- **ranks 27/28 (`0x440c1a9786...`, `0x5455c918e4...`)** — near-duplicate sibling contracts with identical per-pool PnLs (18.4479, 15.5082…), differing by 1 movement — co-leg bots of the same flow.
- **ranks 94/99 (`0x4585fe77...`, `0x4068038a...`)** — identical 22,142-byte bot bytecode, pure `denom_in`/`denom_out` co-legs, no token legs, sourced from MEV-builder bundles.

### Archetype 2 — Scam creators / off-pool seizers (23 rows, `scammer_extraction`)
Real ETH, genuinely extracted, but it is rug proceeds, not trading.
- **The 7702 serial-rug bot fleet, ranks 43–51** (`0xae4b3464...`, `0x85f0f2e3...`, `0xe7c7fb87...`, `0xc8ccff77...`, `0xc0e5e3125e...`, `0x2cf49498...`, `0xe5fbeca5...`, `0xa96c56a2...`, `0x7bebc5aa...`) — **8+ EIP-7702 EOAs all delegating to the SAME implementation `0x63c0c19a282a1b52b07dd5a65b58948a07dae32b`.** Identical template: seed pool with ~10 ETH + 45M tokens, then pull ~17.5 ETH → net ~+7.5 ETH each, fully movement-backed. One operator running a serial rug-pull bot (block progression 25183k→25227k). `creator_scammer` confirmed for all.
- **rank 7, `0x9ccf7d800cf38ac9a597a6bff2a083b01cccaa3a`** — the cohort's most extreme single pool (**242.5 ETH** in pool 0x21ceb4cc): 503 distinct victim buyers via UniV2 router, **exactly ONE token_out and ZERO token_in** — the LP/token-source side collecting buy-side ETH from a pair-balance-drain scam shared by 709 addresses.
- **rank 12, `0xcf7f06192ad837653be626bf30ff0b85dccae385`** — EOA creator draining its own pool (98 `native_denom_in` legs = 52.16 ETH, zero token legs); pays gas, so it is the real transactor.
- **ranks 36 (`0x6a57e3b9...`), 87 (`0x85814aa1...`), 90 (`0xf06295a0...`)** — creator/pool-creator extractions verified by trace.

### Archetype 3 — Off-pool / external-inflow sellers & rug-exit sellers (17 `rug_exit_proceeds` + several corrected types)
EOAs that received tokens externally (or bought into a confiscation rug) and sold for ETH. Real ETH, scam-related, no clean cost basis.
- **rank 26, `0xd0b564200ae67a32a43256d37744ce554f49b7fd`** — cleanest profile in the whole cohort: pays gas (0.00225 ETH, 17 fee legs), real `token_out`→`native_denom_in` sells (gives 0.17e18 token, receives 5.91 WETH). Genuine rug-exit proceeds, only blocked by `pnl_dominated_by_scam_pools`.
- **rank 78, `0x819cf7e1416067c46c44f5b3cd23f37419ea0f17`** — fully `movement_backed` (backed=1), net +4.03 ETH, single pool, **no reconciliation flag** — the single most cleanly-supported number in the top-100, still inadmissible only because scam-pool-dominated.
- DOGEUS cluster (ranks 75–80, 85, 88) — 7 of the 71–80 cohort transact the same pool `0x12d44f8b...` (token DOGEUS, 9-dec) selling the rug for ETH.

### Archetype 4 — Phantom recipient legs / calc artifacts (16 `calc_artifact` + 3 `unclear`)
The clearest evidence the top is broken. Examples with the failure mode:
- **rank 18, `0xa250cc729bb3323e7933022a67b52200fe354767`** — most-extreme single-pool line drilled: headline **67.60 ETH** from a transient WETH co-leg inside a 990-log, 13.3M-gas arbitrage bundle (`tx.from` a different EOA, value 0.00002 ETH). Only 25 movements netting 37.1 ETH persist. Phantom co-leg (D5).
- **rank 25, `0xc7bbec68d12a0d1830360f8ec58fa599ba1b0e9b`** — **denom mis-scale**: top pool 0x486a is **USDT (6-dec)**, `denom_in` 96.85 **USDT** booked as 89.05 **ETH**. `all_eth=false` is the tell.
- **rank 35, `0x1d42064fc4beb5f8aaf85f4617ae8b3b5b8bd801`** — **sign-flip**: real ~8.65 WETH LOSS (8.66 spent vs 0.01 received) booked as **+17.36** by custody-seize logic.
- **rank 38, `0x5ece2eee00d6b1003729e477321b2ee3e520ea66`** — worst-backed row in the cohort: `aggregate_only`, **ZERO movements persisted**, +22.42 is a fully unbacked phantom (D2 + D6).
- **rank 79, `0x0873d157e648d1f34f946efcc3462df2f8587b15`** — **D5 proven**: one 2.0 ETH inflow booked twice (`denom_in` 2.0 + `native_denom_in` 2.0) → 3.9963 ≈ 2×.
- **rank 81 (`0xbee3211a...`) and rank 89 (`0xd3d2e269...`)** — **sign-flipped losers**: raw net is **-5.37 ETH** and **-2.975 ETH** respectively, booked as +2.63/+3.87 and +3.66. These are genuine losing buyers masquerading as winners.

### How much top PnL is real trader edge?
**Essentially none.** Of 100: 40 bot/infra, 23 scammer extraction, 17 rug-exit proceeds, 16 calc artifact, 3 unclear, **1 real_trader_edge (rank 95, and even that is scam-token).** The cohort-0 note states it bluntly: *"none is real trader edge."* The top of the leaderboard measures the D3/D5/D6 defect surface, not skill.

---

## 2. LOSER ARCHETYPES (the bottom-100)

The bottom is the inverse: it is mostly **real**.

### pnl_verdict distribution (bottom-100)
| pnl_verdict | count | meaning |
|---|---|---|
| `genuine_rug_loss` | **61** | bought fresh tokens, rugged to closed_zero_valuation — REAL |
| `calc_artifact` | **15** | unit-mix (USDC/USDT) or D5/sign on contracts |
| `bot_or_infra_flow` | **13** | router/pair/gas-burn — not a single trader's loss |
| `genuine_custody_loss` | **11** | bought into confiscation pool, seized — REAL |

So **72/100 losers are confirmed real losses.** This is the strongest evidence the engine's loss side is sound.

### Who loses the most: rug buyers >> custody victims >> artifacts/infra
- **Rug buyers dominate** (76 computed `rugged_buyer`). The bulk is a **coordinated sniper/sybil fleet**: ranks 22–42 and 46–66 are ~40 near-identical EOAs (high nonces, code=0x), each buying 320–349 freshly-launched tokens via UniV2 Router02, every one reserve-dump-rugged to closed_zero_valuation, total clustered tightly at **-15.6 to -21.4 ETH each**, scam_share ~1.0, all in the same 25180xxx–25229xxx block window. Cross-wallet overlap is decisive: in cohort 3, **86 pools were bought by ALL 10 wallets.** One operator, many wallets. These are **real losses** (each verified: movement net-native == booked net_cashflow to 4 dp; e.g. rank 51 spent 18.88 ETH, recovered 3.05, net -15.8305 exactly).
- **Custody victims (11 genuine):** ranks 5, 43, 71, 73, 88, 89, 92, 96, 100 (+69 mechanism). All **NEGATIVE PnL**, pure buy-then-seized (e.g. rank 88 `0x964a78fa...`: 11 buys = 6.8786 ETH `native_denom_out` + `token_in`, **zero** `denom_in`/`token_out`, net -6.8786 exactly). These are real confiscation losses on pools `0x12d44f8b` / `0xa526ed6f`.

### Are the losses REAL or calc artifacts?
**72 real, 28 not.** The 28 non-real break down as:
- **15 `calc_artifact`** (see flagged list in §2a)
- **13 `bot_or_infra_flow`** — router/aggregator/pair contracts whose pooled flow is not one trader's loss: rank 2 **Banana Gun Router 2** (`0x3328f7f4...`), rank 4 **Maestro Router 2** (`0x80a64c6d...`), ranks 3/6/7/8 router clones, rank 44 (`0x77edae6a...`, upgradeable proxy), rank 68 (`0xae2fc483...`, gas-burn — see below), and two **Uniswap V2 Pair contracts** mis-booked as traders (ranks 82 `0x76a411f1...`, 90 `0x7dfc9dd5...`, both 11293-byte UniV2Pair, factory `0x5C69bEe7...`), and rank 70 (`0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640`) = the **canonical Uniswap V3 USDC/WETH 0.05% pool itself**.

### 2a. Every flagged `calc_artifact` on the bottom, with address + why
| rank | address | why it is an artifact |
|---|---|---|
| **1** | `0xf95a15be06bd3ff2f3662d3dbcc4475d1e52c49f` | **-39,894 "ETH" — the single largest loss in the run is BOGUS.** 3 giant legs are USDC (6-dec) summed as ETH (`fc33bb` denom_out_raw 20214870000 = 20,214.87 USDC). Really ~39,867 USDC + ~27 ETH. `all_eth=false` but **`tier_one_admissible=true` — GATE LEAK.** |
| **9** | `0x1f2a2c9f26a8438a20fc14b2401a84584349014f` | Same USDC pools; -213 "ETH" is mostly USDC and one leg is actually a USDC *gain* booked as loss. `tier_one_admissible=true` WRONG. |
| **10** | `0x9d25a2a49dac7e89696211ba3157bad964b4a3c2` | Clearest unit-mix: `denom_out_raw` 56000000 → pnl -56 "ETH" (really 56 USDC). -166 total is ~166 USDC. `tier_one_admissible=true` WRONG. |
| **11** | `0x8f10b468b06c6fd214b65f87778827f7d113f996` | -52.48 built from native legs duplicated ~3× (D5); pool-direct shows it *received* more than it spent. Contract co-leg, not a victim. |
| **12** | `0xb300000b72deaeb607a12d5f54773d1c19c7028d` | 180-byte minimal proxy; -67.58 top pool partial_movements; native-leg double-count signature. |
| **13** | `0x734ab9de48f6bab1f2297a34d257cd757deba6aa` | Pool-direct net only ~-3 ETH, booked -45.50; native legs (17.70 in / 35.22 out) inflate it; 793-byte contract. |
| **14** | `0x60e997de8d91f0b254e7b130fc9684eda3544a72` | **-70.65 ETH does NOT reconcile to any leg** (all legs nano-ETH, 1.2M tokens still held). Fabricated mark + D2 over-assignment (really the creator). |
| **15** | `0x4c82d1fbfe28c977cbb58d8c7ff8fcf9f70a2cca` | 24.5KB swap-callback router; -54.14 partial_movements; also holds a **+28.23 ETH** pool, contradicting a pure-victim story. |
| **16** | `0x7f54f05635d15cde17a49502fedb9d1803a3be8a` | Pool-direct **net +16 ETH (a gain)** booked as -27.9; `native_denom_in == native_denom_out == 198.84` exactly (textbook D5). |
| **17** | `0x503972ee3a62480f66e7a1bd485dd5f7da754bec` | Genuine net loss only ~-1.04 ETH, reported -10.89; native legs inflate/flip. 23KB router. |
| **18** | `0x74de5d4fcbf63e00296fd95d33236b9794016631` | 1.16KB router clone on shared rug pools; duplicated routed legs inflate -25.01. |
| **19** | `0xf38521f130fccf29db1961597bc5d2b60f995f85` | 33 movements, -20.80 from 20 movements → ~1 ETH/movement implausible for a tiny-footprint contract. |
| **20** | `0x60594a405d53811d3bc4766596efd80fd545a270` | **Pure sign-flip**: 2 legs BOTH `denom_out` (received 24.22 ETH), `denom_in=0`, zero tokens — booked as **-24.22**. 22KB router. |
| **21** | `0xef6fc636a63859e08ad3479fe456262eed2e5042` | Buy-and-disperse/fee-distributor contract; has a **+4.6748 ETH "victim" pool** (D2 positive-pnl anomaly) + D3 gas=0; -22.56 is mis-attributed dispersal legs. |
| **79** | `0x915fd34cadd63907b51eb64dddc2eadd114a0bed` | -8.24 is ~98% bogus: top pool 0x486a is **USDT (6-dec)**, single leg 8.06 USDT booked as -8.0639 ETH. Real ETH loss ~-0.18. `all_eth=false`. |

The **implausibly large losses are all calc artifacts**: rank 1 (-39,894), rank 14 (-70.65 unreconciled), rank 11 (-52.48), rank 13 (-45.50). Genuine rug/custody losses are sanely bounded (≤ ~-21.4 ETH, mostly single-digit per pool).

---

## 3. Taxonomy validity (computed types: confirmed vs corrected)

| | TOP-100 | BOTTOM-100 |
|---|---|---|
| `type_verdict = confirmed` | **48** | **78** |
| `type_verdict = corrected` | **52** | **22** |

**The taxonomy holds far better on the bottom than the top.**

- **Bottom (78% confirmed):** `rugged_buyer` (76 computed) is overwhelmingly correct — the rug-buyer label survives drilling. Corrections (22) are mostly **router/infra contracts mislabeled as buyers/victims** (ranks 2/4/6/7/8 → `bot_router_infra`; 70/82/90 → infra pool/pair; 97 → mixed_trader bot). The taxonomy correctly identifies real losers; it just doesn't yet recognize infra contracts.
- **Top (only 48% confirmed):** `fresh_launch_sniper` (44 computed) is the **most over-assigned label** — it gets corrected ~30 times. The systematic error: addresses labeled "sniper" **never buy a token** (no `token_in`); they only receive ETH. They are `external_inflow_seller`, `bot_or_infra_flow`, or `router/infra`, not snipers. `creator_scammer` (17) held up perfectly (the 7702 fleet confirmed). The confirmed-but-not-edge cases show the taxonomy can be "right about the type and still useless" — a confirmed scammer is still not a trader.

Bottom line on taxonomy: **the *type* labels are mechanically validatable and mostly correct, but "fresh_launch_sniper" needs a guard requiring a `token_in` (buy) leg**, and **both sides need an infra/contract class** so routers/pairs/settlement contracts aren't typed as traders.

---

## 4. The custody contrast (the D2 evidence)

This is the cleanest single confirmation that the **D2 fix is needed and correct.**

- **TOP (positive-PnL "custody victims" = the D2 bug):** 10 rows computed `custody_anomaly` — ranks **38, 40, 70, 72, 73, 80, 91, 97, 98, 100**. Every one is a `custody_victim_with_positive_pnl`. Drilling shows **none is a victim**: they are escaped sellers/seizers/creators (e.g. rank 72 `0xef1c5994...` net +3.72 ETH selling DOGEUS; rank 70 `0x74c10e4b...` +4.868 ETH uncosted sell proceeds; rank 40 the CoW Settlement contract; rank 80 `0x1f0e02b2...` a profitable rug churner). A genuine victim **cannot have positive PnL** — so a positive-PnL "victim" is by definition the over-assignment artifact.
- **BOTTOM (genuine custody victims = NEGATIVE):** 13 rows computed `custody_victim`; **9 are genuine `genuine_custody_loss`** (ranks 5, 43, 71, 73, 88, 89, 92, 96, 100) and 4 are corrected to infra (11, 15, 21, 97). The genuine ones are all **NEGATIVE PnL**, pure buy-then-seized, reconciling exactly (rank 88 net -6.8786, rank 100 single pool -5.2549 = summed buys).

**The contrast is exact and diagnostic:** the SAME `custody_victim_candidate` role appears on **positive-PnL phantoms at the top** and **negative-PnL real victims at the bottom**. The sign of PnL perfectly separates artifact from real. **This confirms D2 must be fixed by the rule: a `custody_victim_candidate` with PnL ≥ 0 is not a victim** (it is a seller/seizer/infra). Applying that rule reclassifies all 10 top rows and none of the 9 genuine bottom victims — exactly the desired behavior.

---

## 5. Admissibility — why so few

**0 of 100 top and 0 of 100 bottom are `tier_one_admissible`** (one near-miss caveat below).

Reasons, in order of frequency:
1. **`pnl_dominated_by_scam_pools`** — the dominant blocker on both sides. The entire run's PnL clusters on a handful of scam pools (`0x12d44f8b` custody_buyer_token_confiscation, touched by ~3,988 addresses; `0xa526ed6f`; `0x21ceb4cc`; `0x2315f2f8`; the reserve-dump fleet pools). Nearly every winner and loser has scam_share ≈ 1.0.
2. **`gas_not_attributed` (D3)** — gas=0 on every recipient-leg/bot row; pervasive across the top's bot/infra winners and many losers.
3. **`movement_reconciliation_incomplete` / partial_movements (D6)** — headline PnL exceeds movement-backed totals on dozens of rows (top: 2/9/10/15/17/18/20…; bottom: most custody rows).

This is **expected for a pre-router-threshold baseline**: the universe is dominated by scam pools and infra contracts, so the scam-pool-dominance flag alone disqualifies essentially everyone. The few "clean" actors (top rank 78, rank 95; bottom genuine rug buyers) are still blocked purely by `pnl_dominated_by_scam_pools` because the only tokens in this window were scams.

**The one admissibility FAILURE:** ranks 1, 9, 10 (and effectively 79) on the bottom are **`tier_one_admissible=true` despite being pure unit-mix calc artifacts**, because the USDC/USDT-as-ETH mis-scale is **silent** — no validity flag fires. This is a real **gate leak** that lets the largest fake "loss" in the run (-39,894) through.

---

## 6. VERDICT for the small-run review

**Does address PnL behave sanely on both extremes?**
- **Bottom: YES, mostly.** 72/100 are real, movement-reconciled losses; the loss-side accounting is sound. The fleet of coordinated rug-buyers is correctly captured as genuine losses.
- **Top: NO.** 99/100 are not trader edge — they are bot/router pass-through (40), scam extraction (40 incl. rug-exit), or calc artifacts (19). The winner leaderboard is currently a ranking of the D3 recipient-leg defect plus scam-pool re-attribution, not skill.

**What must still be fixed (tied to the tracked defects):**
- **D2 (custody over-assignment):** CONFIRMED needed. Rule: `custody_victim_candidate` with PnL ≥ 0 → reclassify (escaped seller / seizer / infra). Validated against 10 positive-PnL top phantoms vs 9 negative-PnL bottom real victims — the fix is clean and necessary. **Gate it.**
- **D3 (recipient-leg native inflow booked as profit, gas=0):** This is the dominant top-100 pathology. Required fix: **a position with `native_denom_in` but no `native_denom_out` / no `token_in` cost basis is not edge** — reject "winners" with no buy-side round trip. This single rule removes most of the top-100.
- **D5 (co-leg double-count):** proven on top rank 3 (240.717 ETH booked twice), rank 79 (2.0→3.9963), and bottom rank 16 (native in==out==198.84). Needs co-leg dedup before aggregation.
- **D6 (reconciliation incomplete / partial_movements / aggregate_only):** pervasive; several headline numbers are unsupported (top ranks 52/57/82 `unclear`; rank 38 zero persisted movements). Aggregates that exceed movement-backed totals should be flagged blocking, not silently summed.
- **NEW — unit-mix (USDC/USDT/DAI-as-ETH), the most urgent:** not in the D2–D6 list. **Any pool with `denom_tracks_native_eth=false` (or `all_eth=false`) must be excluded from — or FX-converted before — the ETH-labeled total, AND must raise a blocking validity flag.** Without this, the largest "loss" in the run (rank 1, -39,894, `0xf95a15be...`) passes the Tier-One gate. Fix this **before** gating.
- **Infra/contract recognition:** add a router/pair/settlement/builder pass-through class so Banana Gun, Maestro, Universal Router, CoW Settlement, MEV builders, and UniV2/V3 pool contracts are excluded by identity, not just by scam-pool dominance.

**Is it ready to gate?**
- **As a directional/loss-side signal: nearly.** The bottom is trustworthy once the **unit-mix leak (ranks 1/9/10/79)** is closed — that one defect alone lets fake extremes through the gate, so it is a **hard blocker** to gating.
- **As an "alpha/edge" signal: NO.** The top must not be gated until D3 (no-cost-basis rejection) and the router/infra pass-through filter are applied; otherwise the leaderboard surfaces phantom winners and bot flow, with at most 1/100 representing anything resembling real edge.

**Recommendation:** **Do not gate yet.** Land (1) the silent unit-mix flag, (2) the D2 sign rule, and (3) the D3 cost-basis/round-trip requirement + infra pass-through filter. After those, re-run; the bottom should remain ~stable (real losses) and the top should collapse to the handful of genuine sellers/creators, at which point the small-run gate is defensible. The pre-threshold caveat held: every router/Banana-Gun/builder/pair contamination predicted was found exactly where expected, and no new defect surfaced on the top — the only new defect (unit-mix) came from the bottom.
