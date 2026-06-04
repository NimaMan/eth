# Scammer Analytics — Design & Build Spec

This is the authoritative spec for the scammer-analytics product. It defines the
tone, the entity model, every statistic and how to compute it, the data
contracts, and the page structure. Implementation agents build against THIS doc.

See also `README.md` (Objective — Follow the Money) and the per-case READMEs.

---

## 1. Tone / objective (non-negotiable)

The objective is **actionable intelligence to clean up the chain** — flag and
track the malactors, follow the stolen money to its off-ramp, and produce a
packet that an average person can read AND that is usable to escalate to law
enforcement and to contact the exchanges that received the funds. Use anything
that helps (PnL, token/pool state, the fund-flow tracer, the CEX catalog,
mempool/risk events).

Concretely it is a **law-enforcement-grade "follow the money" platform**. For any
scammer (or operation), show in plain terms — for someone with NO crypto
background — **how the scam happened, how much was stolen, and exactly where the
money went** (which exchanges it was cashed out through, and what is still
recoverable / freezable). The money trail and its actionability are the headline;
engineering tables are demoted supporting evidence. If a non-expert cannot answer
"where did the money go?" in under a minute, the page failed.

Every number on the product carries a **confidence tier** and **provenance**
(see §6). Nothing is asserted as attribution without on-chain evidence.

---

## 2. Entity model (SCAMMER-CENTRIC)

Primary unit = the **scammer** (operator), keyed by their operator wallet(s).
Levels, from broadest screening to confirmed operation:

0. **Address (first-degree participant)** — ANY active address, screened for scam
   participation: its **scam-trade ratio** (fraction of its pool positions that
   are scams), # scam pools, roles, mechanisms touched, and the scam cases it
   appears in. The upstream "is this address dirty?" assessment — most addresses
   are participants/victims, a few resolve to scammers. Computed from the
   token-PnL run (per-address scam-ratio). See §3.5.
1. **Case** — one incident: token + pool + drain. Lives under `cases/<case_id>/`
   (`case.toml` + `artifacts/` + `reports/`). Already exists.
2. **Scammer** — one operator, keyed by their **funding root** (the durable
   identity), with their cases as a **tree** beneath it. A scammer does NOT reuse
   one operator wallet across cases: the observed pattern is *fund a fresh address
   → run one scam → abandon it → repeat from the same funder*. So cases attribute
   to a scammer primarily by the **shared funder** (and secondarily shared cash-out
   path / forwarder / identical token bytecode), NOT by a reused `suspect_address`.
   Grouping cases by `suspect_address` yields one case per scammer — the rollup
   must group by the funder tree. See §5.3.
3. **Cluster / operation** — a *set* of scammers linked by shared infrastructure
   (§5.3). Lives under `clusters/<cluster_id>/`.
4. **Aggregate / landscape** — everything, for compliance and trends; the product
   **home page**.

Page hierarchy: **Landscape (home) → Scammer → Case**, plus **Cluster** detail and
the cross-cutting **Address** screening page (`address/:address`). The
home/landscape must surface scammers, the per-exchange exposure, and the
scam-heavy addresses as the entry point (not bury them under analytics tables).

---

## 3. General stats (from PnL + token-state) — "how bad / how"

Per case, rolled up per scammer and per cluster, and aggregated.

| Stat | Definition | Source / how to compute | Confidence |
| --- | --- | --- | --- |
| Value stolen (operator take) | Operator's realized denom PnL across their pools | `eth_token::pnl` semantic export (`denom_cashflow`, `pnl_proxy_denom`) for operator wallets; the token-pnl run in `eth_db` | tool_derived |
| Victim loss | Σ confiscated buyers' expected balance valued at pool price | custody reconciliation (`buyer_token_outcomes.json` summary) | tool_derived |
| Victims | # pool-out buyers; # fully drained (`confiscated`); loss distribution | `buyer_token_outcomes.json` | on_chain_fact (counts) |
| Tokens launched / pools | # distinct tokens & pools attributed to the operator | token state + creator/owner attribution | tool_derived |
| Scam mechanisms | Distinct `scam_mechanism` labels across the operator's pools | pool flags / `scam_mechanism` (`holder_balance_backdoor_drain`, `custody_buyer_token_confiscation`, `direct_lp_liquidity_removal`, …) | tool_derived |
| Lifecycle timing | block deltas deploy→trading→drain→cash-out | `key_txs` + token state | on_chain_fact |
| Detection coverage | Did our LIVE detector emit a holder-balance-drain / confiscation risk event for this case? | risk events vs the drain block (the "Ask-1" detector) | tool_derived |

### 3.5 Address first-degree assessment (screening)

For ANY address (not only confirmed scammers), a first-degree scam-participation
profile — the "is this address dirty?" screen that sits upstream of the
scammer/cluster pipeline:

- **scam-trade ratio** — fraction of the address's pool positions that are scams.
- # scam pools / total pools, trade count, total absolute flow.
- roles (buyer / creator / funder / control / remover), scam mechanisms touched,
  pool labels.
- the **scam cases / scammers it appears in** (first-degree links).

Source: the token-PnL run, per address. `risk_atlas/src/db/reader.rs::scammer_address_distribution()`
already computes the scam-ratio + roles + mechanisms for the top-N; a **per-address
variant** (the same scam-ratio query filtered to one address, joined to the cases
that address participates in) backs the page. Confidence: `tool_derived`
(PnL-run-based); the case links are `on_chain_fact`. Page: `address/:address`
(§9). A high-scam-ratio address is a **scammer candidate** that feeds §2 level 2.

---

## 4. Fund-flow / exchange stats — "where the money went" (the headline)

Produced by the cash-out tracer (`eth_token` example
`custody_suspect_cashout_trace` → `artifacts/tx_fund_flow/suspect_cashout_trace.json`),
multi-asset (ETH/WETH + USDC + USDT), value-conserving (taint), swap-carry,
service-terminal. Per scammer and aggregated.

| Stat | How to compute | Source |
| --- | --- | --- |
| Total value traced out | `summary.total_value_out_traced_eth` | trace artifact |
| **Per-exchange cash-out** ("Binance X ETH, OKX Y") | Group `terminals[]` by `exchange`, sum `total_value_received_eth` / `received_by_asset` | trace `terminals[]` + CEX catalog (`reth_chain_query::common_addresses::cex`, 2,776 named addrs) |
| **Aggregate per-exchange exposure** | Σ per-exchange across ALL scammers' traces → compliance-target table | read-model rollup |
| Funding source exchange (KYC entry) | `funding_sources[]` where `source_kind == "cex"` | trace artifact |
| Cross-chain / bridge leakage | terminals/leads whose label is a bridge → cross-chain follow-up flag | trace nodes (label bridges; today many are `unlabeled` leads) |

### 4.1 Recoverability framing (REQUIRED v1)
Split the traced value into actionability buckets (computed from the trace's
terminal/lead node kinds):

- **At an exchange (freeze-able via the exchange)** — value reaching a CEX
  terminal. Action: compliance contact / freeze request.
- **In a wallet (still traceable, potentially recoverable)** — value sitting at an
  unexpanded non-service wallet lead (`kind == unlabeled`, has tainted value, not
  yet moved on). Action: monitor / trace deeper / civil freeze.
- **Bridged (cross-chain follow-up)** — value into a known bridge.
- **Destroyed** — value to a burn address.
- **Off-ramped / gone** — value that left a CEX or entered a mixer (best-effort).

Implementation: extend the tracer to emit a `recoverability` summary
(`{at_exchange_eth, in_wallet_eth, bridged_eth, destroyed_eth}`) derived from final
node classification, plus per-node `recoverability_bucket`.

---

## 5. The four extra aspects (all in v1)

### 5.1 Recoverability — see §4.1.

### 5.2 Per-exchange aggregate exposure
A landscape table: for each exchange, total scam value that flowed to it across
all scammers (+ # scammers, # cases). This is the compliance contact list. Source:
read-model rollup over all `suspect_cashout_trace.json` terminals.

### 5.3 Cluster attribution (how we link scammers into operations)
Link signals, ranked by strength:
1. **Shared cash-out exchange DEPOSIT address** (same deposit addr across operators) — STRONG.
2. **Shared funder wallet** (same address seeded multiple operators) — STRONG.
3. **Repeated creator / identical token bytecode** — STRONG.
4. **Shared forwarder contract / final sink** — MEDIUM.
5. **Tight timing / same launch venue** — WEAK (corroborating only).

Implementation: a cross-case clustering pass (new tool) that joins, across cases,
the `address_clusters.json`, `suspect_cashout_trace.json` nodes/funding, creator
addresses, and forwarder sinks; builds a link graph; emits `clusters/<id>/` with
members + the linking evidence + a confidence tier. Never assert a cluster on
WEAK-only evidence.

### 5.4 Confidence tiers + provenance — see §6.

---

## 6. Confidence & provenance convention (product-wide)

Every surfaced statistic is `{ value, confidence, provenance }`:
- `confidence ∈ { on_chain_fact, tool_derived, inference }`.
  - `on_chain_fact`: directly from chain data (a transfer, a balance, a count of txs).
  - `tool_derived`: computed by our accounting/tracer (PnL, taint value, classifications).
  - `inference`: heuristic attribution (cluster links, "operator take" estimates).
- `provenance`: `{ source, run_id?, block_range?, method }` — the artifact/run and how it was computed, so a packet is reproducible and court-credible.

The UI badges confidence (fact = solid, derived = outlined, inference = dashed/"lead").

---

## 7. Aggregate / landscape stats

Across all scammers: total stolen, total traced to exchanges, **top exchanges by
exposure** (§5.2), top scam mechanisms, # scammers, # clusters, # victims,
detection-coverage %, and a simple time trend (cases per week). Source: read-model
rollup over case artifacts + the PnL run.

---

## 8. Data contracts / API

Single endpoint family (served by `eth_chain_server`, path
`/api/v1/eth/analytics/risk-atlas/scammer-analytics`, file-backed by
`read_models/analytics/scammer.rs`). Extend the payload to add, alongside today's
`cases[]`:

- `scammers[]` — scammer-centric rollups (group cases by operator wallet set):
  per-scammer general stats (§3), fund-flow stats (§4), recoverability (§4.1),
  cluster membership, and confidence/provenance per stat.
- `exchanges[]` — aggregate per-exchange exposure (§5.2).
- `clusters[]` — cluster list with members + linking evidence + confidence.
- `summary` — landscape stats (§7) with confidence.
- each `cases[].suspect_cashout_trace` already carries the per-case fund flow.
- **address screening** (§3.5) — a per-address endpoint
  `/api/v1/eth/analytics/risk-atlas/scammer-analytics/address/{address}` (or reuse
  the existing `/eth/tokens/api/traders/{address}`) returning one address's
  scam-trade ratio, scam pools, roles, mechanisms, total flow, and the scam
  cases/scammers it appears in. Backed by a per-address variant of
  `risk_atlas/src/db/reader.rs::scammer_address_distribution()`.

Keep backwards compatibility (existing `cases[]` fields stay). A per-scammer and
per-cluster detail can be the same endpoint filtered client-side, or add
`/scammers/{id}` and `/clusters/{id}` sub-routes if payload size demands it.

---

## 9. Page structure (UX)

All pages obey §1 (follow the money; non-expert first; tables in a collapsed
"evidence" fold).

- **Landscape / HOME** (`/eth/risk-atlas/scammer-analytics`): the product home,
  covering the three pillars at a glance — aggregate verdict (total stolen, total
  to exchanges, recoverability split), **top-exchanges exposure** table (pillar 3
  compliance targets), the **scammer list**, and the **scam-heavy address**
  distribution (the §3.5 screen, top-N). Scammers + addresses are the first thing
  seen (not buried under analytics tables).
- **Scammer detail**: verdict banner → **money-flow path** (visual, the centerpiece)
  → "what happened" step story → "where the money went" (per-exchange + recoverability)
  → victims summary → evidence fold. (See `/tmp/scammer_redesign_spec.md` for the
  single-scammer page redesign brief.)
- **Address screening** (`/eth/risk-atlas/scammer-analytics/address/:address`): the
  first-degree assessment (§3.5) for ANY address — scam-trade ratio, # scam pools,
  roles, mechanisms, total flow, and the scam cases/scammers it appears in. The
  entry screen; a high-ratio address links forward to its scammer/case pages.
- **Cluster detail**: the linked scammers, shared infrastructure, combined money
  flow + combined per-exchange exposure.

---

## 10. How each piece is produced (build order)

1. **Per-case artifacts** (exist / extend):
   - buyer outcomes: `cargo run -p eth_token --example custody_session_scammer_case_report`
   - fund-flow trace: `cargo run -p eth_token --example custody_suspect_cashout_trace`
     (extend with recoverability §4.1 + bridge labeling + confidence/provenance).
2. **Read-model aggregation** (`scammer.rs`): scammer-centric rollups, per-exchange
   exposure, recoverability summary, landscape stats, confidence/provenance (§8).
3. **Cluster pass** (new tool): cross-case clustering (§5.3) → `clusters/`.
4. **Frontend** (kimi, assessed by Playwright): landscape page, scammer-detail
   redesign, cluster page (§9).

---

## 11. Working model

- **Docs & orchestration**: maintained by the lead (this doc is the source of truth).
- **Implementation agents**: each owns a slice and does its **backend (Rust)**,
  drives **kimi** for the matching **frontend**, and **assesses kimi's output with
  Playwright** (render the live page at `http://100.96.34.94:40020/...`, screenshot,
  check the money-flow reads clearly for a non-expert). Backend cargo builds share
  one target dir — sequence heavy Rust builds to avoid contention.
- Always separate **on-chain fact / tool-derived / inference**; attach the tx hash,
  artifact path, or run_id behind every claim.
