# Scammer Analytics

Actor-centric investigation workspace for Risk Atlas.

`scam_analytics/` labels token and pool behavior. `scammer_analytics/`
follows wallets and infrastructure: creators, funders, removers, forwarders,
final sinks, repeated bytecode, repeated token launches, and reportable
entities.

## Objective — Follow the Money

The objective is **actionable intelligence to clean up the chain**: flag and
track the malactors, follow the stolen money to its off-ramp, and produce output
usable to escalate to law enforcement and to contact the exchanges that received
the funds. The page is how we deliver it — it must let **someone with no
crypto/on-chain background follow the money**: see, in plain terms, how a scam
happened and where the stolen funds went, hop by hop, until they reach a
centralized exchange (or a confidence-bounded dead end). Every design and data
decision serves that goal; we use anything that helps.

The page must read as a simple narrative, not a wall of tables:

1. **Who** — the scammer (suspect) and what they did.
2. **How** — the scam mechanism, as a short timeline of key transactions.
3. **The money out** — a clear, visual fund-flow path from the scammer to the
   exchange they cashed out at (value-conserving, so we follow *their* money, not
   an intermediary's throughput), with the off-ramp named.
4. **The victims** — who lost what.
5. **What to do next** — the law-enforcement / compliance follow-up (exchange to
   contact, deposit addresses to confirm).

If a non-expert cannot answer "where did the money go?" in under a minute on this
page, the page has failed its objective. Dense engineering tables are supporting
evidence, not the headline — the money trail is the headline.

The full product spec — scammer-centric entity model (case → scammer → cluster →
landscape), every statistic and how to compute it (general PnL/token-state stats,
fund-flow/per-exchange stats, recoverability, cluster attribution, confidence &
provenance), the data contracts, and the build order — lives in
[`DESIGN.md`](DESIGN.md). Implementation is done by agents that build the backend,
drive the `kimi` CLI for the frontend, and assess it with Playwright.

## Rust Flow

```text
case.toml
  -> eth_risk_atlas scammer-case
  -> tx_processor ProcessedTransaction
  -> tx_fund_flow directed ETH/token movements
  -> case artifacts and report packet
```

Run a case:

```bash
cargo run -p eth_risk_atlas -- scammer-case \
  risk_atlas/scammer_analytics/cases/eth_0x9d58c75a_e_pair_25181124/case.toml
```

## Folder Structure

| Folder | Purpose |
| --- | --- |
| `cases/` | One folder per investigated incident (the source of truth). |
| `scammers/` | Primary entity — per-operator roll-up of their cases. |
| `clusters/` | Operations — sets of linked scammers (shared infrastructure). |
| `addresses/` | First-degree screening of any participant address (scam-trade ratio). |
| `exchanges/` | Per-exchange aggregate exposure + compliance-contact targets. |
| `schemas/` | Case, cluster, address, and report contracts. |
| `pipeline/` | Runbook: how each artifact is produced, in order, with provenance. |
| `packets/` | Generated, exportable law-enforcement & exchange-contact bundles. |
| `detector_prototypes/` | Candidate actor-level detector notes before promotion. |

Each folder carries a `README.md` stating its objective. The full spec is
[`DESIGN.md`](DESIGN.md).

Generated case outputs live under each case's `artifacts/` and `reports/`
folders. Keep large regenerated files out of normal commits unless they are
needed as fixtures.
