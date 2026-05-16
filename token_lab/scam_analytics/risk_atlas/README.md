# Risk Atlas

Rust-owned read model for the `/eth/tokens/analytics/risk-atlas` page.

Risk Atlas lives under `token_lab/scam_analytics/` because it is a lab surface
for scam labels, scam mechanisms, active-horizon targets, and model-readiness
analysis. It does not own token or pool source-state feature construction.

## Boundary

```text
eth_token::token_analytics
  -> source active observations and as-of feature families
token_lab/scam_analytics
  -> labels, target joins, review queues, model experiments
token_lab/scam_analytics/risk_atlas
  -> durable aggregate DB and page-ready story/read model
eth_chain_server
  -> API endpoint that reads the Risk Atlas DB
Asena
  -> renders the page; no flat-file parsing or feature logic
```

The current 100K distribution report can seed this DB while the Rust feature
pipeline is being finalized. Once the source feature rows are fully produced by
`eth_token::token_analytics`, the importer should read those typed rows instead
of markdown reports.

## Current State

The surrounding `scam_analytics/` folder currently contains:

- human-auditable scam label rules in `labels/`;
- model-row contracts and leakage rules in `features/`;
- generated distribution and validation reports in `artifacts/reports/`;
- this Rust crate for DB migrations, imports, and page-ready views.

Risk Atlas is the cleanup layer that turns those outputs into a compact DB read
model for a high-level page: launch surface, scam type distribution, time to
scam, active-horizon targets, review queues, and trading relevance.

## Page Story

Risk Atlas is meant to answer "what is going on?" from the highest level down.
The first section should be the launch and eligibility surface, not scam labels.
All later distributions should be framed over the eligible cohort unless the
page explicitly says it is showing the full observed universe.

The launch flow should make these questions visible:

- observed pools in the range;
- supported-denom/protocol pools;
- pools that crossed the liquidity floor;
- pools that became cohort-buyable and cohort-sellable;
- pools that entered the eligible cohort;
- eligible pools that are currently active versus eligible-risk;
- ineligible pools by exclusion reason;
- the same funnel over time.

Eligibility itself is owned by `alpha/pool_classification`. The current contract
is intentionally simple: supported quote, current denom liquidity above the
configured floor, cohort buy availability, and cohort sell availability.
Eligibility is first satisfied at some point in the pool lifetime, not
necessarily at pool creation. Once a pool is eligible, it remains eligible;
later failures are outcomes inside the eligible cohort. Risk Atlas should not
reimplement these rules in the frontend.

The DB contract stores this first filter in `risk_atlas_pool_eligibility`.
Future pool-atlas generation should write one row per pool with:

- `eligible`: the boolean filter used by all downstream analysis;
- `eligibility_block`: the first block where the current as-of-block pool state
  satisfied the criteria;
- `eligibility_liquidity`: the liquidity observed at entry.

All scam-rate, time-to-scam, active-target, and model-row outputs should be
built from `risk_atlas_pool_eligibility.eligible = true` unless a page section
explicitly says it is reporting the full observed universe or ineligible
exclusion reasons.

## Layout

```text
risk_atlas/
  migrations/
    001_create_risk_atlas.sql
  src/
    atlas/      # section semantics and model-readiness vocabulary
    api/        # page-facing view structs
    db/         # schema structs, reader, writer, migrations
    ingest/     # source layout/contracts for current artifacts and future rows
    config.rs
    lib.rs
    main.rs     # small migration/schema CLI
```

Local database dumps or generated snapshots belong under `risk_atlas/data/`,
which is ignored.

## Commands

Print the SQL schema:

```bash
cargo run -p token_lab_scam_risk_atlas -- schema
```

Apply migrations to the code-default Postgres database:

```bash
cargo run -p token_lab_scam_risk_atlas -- migrate
```

Import the current 100K distribution report:

```bash
cargo run -p token_lab_scam_risk_atlas -- import-report
```

The API/server integration should use `RiskAtlasReader` from this crate rather
than reading report artifacts directly.
