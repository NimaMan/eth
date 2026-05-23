# Risk Atlas Execution Modes

Risk Atlas has two generation modes and one read mode. Pick the mode from the
user goal before starting a run.

## Decision Matrix

| User goal | Mode | Use server? | State refresh rule | Output |
| --- | --- | --- | --- | --- |
| Build model data for 100K, 500K, or larger ranges | Headless generation | No | No frontend state; only cheap local progress/logs | Risk Atlas DB rows and aggregates |
| Rebuild a dataset for training/evaluation | Headless generation | No | Batch writes only | Risk Atlas DB + model-ready rows |
| Inspect a range in Asena token builder | Server-backed range builder | Yes | Cheap progress every block; heavy snapshots every configured interval and at completion | Range read models + optional Risk Atlas DB export |
| Debug one token/pool visually | Server-backed range builder or live page | Yes | Snapshot/read-model refresh is acceptable | Token/pool pages |
| Live trading decisions | Live trading event path | Server process may host state, but clients should use committed events/state | Every committed block, after full block apply | Trading/risk state, not page DTOs |
| Risk Atlas page display | DB-backed read mode | Server reads DB only | No block processing | Page story and decision-question read models |

## Mode 1: Headless Generation

Use this for large modelling datasets. The run should not depend on
`eth_chain_server` HTTP state, Asena pages, or range-builder token/pool DTOs.

Target flow:

```text
processed-block source
  -> eth_token block apply
  -> token_analytics active observations
  -> Risk Atlas batch writer
  -> risk_atlas_* tables
  -> aggregate/target rebuild
```

Rules:

- Do not materialize frontend token/pool tables every block.
- Do not keep the whole range in `RangeIndexState`.
- Write observations in bounded batches.
- Emit only cheap progress/log records during the run.
- Derive targets and decision-question aggregates from DB rows.

Use this mode when the user asks for:

- `100K`, `500K`, or larger data generation;
- model training/evaluation;
- scam-label distributions;
- target/base-rate distributions;
- a run that does not need immediate token-builder inspection.

## Mode 2: Server-Backed Range Builder

Use this when the user needs the frontend token range builder to inspect tokens,
pools, or active observations while or after a range is built.

Target flow:

```text
POST /api/v1/eth/ranges
  -> range hot apply loop
  -> cheap progress update every block
  -> read-model snapshot refresh every configured interval
  -> terminal snapshot refresh at completion
  -> optional Risk Atlas DB export
```

Rules:

- Update cheap progress every block.
- Refresh heavy read models on a configured interval, not every block.
- A default interval of `1000` blocks is the first practical target for large
  server-backed builds.
- Always refresh final token/pool/surface/Risk Atlas export state when the run
  reaches a terminal state.
- Frontend and agent clients must tolerate stale range snapshots during active
  builds.

Use this mode when the user asks for:

- a token range builder run;
- frontend access to tokens from a range;
- visual token/pool debugging;
- a smaller smoke run intended for page inspection.

## Mode 3: DB-Backed Read Mode

Use this when the run already exists in the Risk Atlas DB and the user wants to
inspect the page or compare generated cohorts.

Target flow:

```text
risk_atlas_* tables
  -> RiskAtlasReader
  -> /api/v1/eth/analytics/risk-atlas
  -> Asena
```

Rules:

- No block processing happens in this mode.
- No CSV/report parsing in the frontend.
- Page questions and story sections come from DB-backed read models.

## Folder Ownership

Current layout:

```text
risk_atlas/
  docs/
    execution-modes.md      # run-mode and agent decision rules
  migrations/
    001_create_risk_atlas.sql
  src/
    atlas/                  # aggregate sections, decision questions, story
    api/                    # page/API view structs
    db/                     # schema structs, reader, writer, migrations
    ingest/                 # source adapters for reports and future row feeds
    config.rs
    lib.rs
    main.rs                 # migration/schema/import CLI
```

Target generation split:

```text
risk_atlas/src/generation/
  headless.rs               # large range generation without HTTP server state
  server_export.rs          # export/finalize from a completed server range
  batch_writer.rs           # bounded observation batch writes
  targets.rs                # active-observation target finalization
  progress.rs               # cheap generation progress records
```

Create this code folder when we implement the headless generator. Until then,
keep generation contracts in this document and avoid pushing generation logic
into `eth_chain_server` read models.

## Agent Rules

An agent should choose:

- **Headless generation** when the request is about model data, large ranges,
  distributions, training rows, or evaluation.
- **Server-backed range builder** when the request explicitly needs Asena/token
  builder access to the produced tokens or pools.
- **DB-backed read mode** when the request is about reading or redesigning the
  Risk Atlas page from an already imported run.
- **Live trading event path** when the request is about trading decisions,
  execution gates, or per-block live risk state.

If both frontend inspection and model data are required, run the model-data
generation headlessly first, then expose the DB-backed result to the frontend.
Only use the server-backed range builder for the subset that needs interactive
inspection.
