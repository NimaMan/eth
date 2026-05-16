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
