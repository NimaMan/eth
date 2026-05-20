# Strategy Validation

This module validates persisted alpha strategy results before we trust PnL,
strategy comparisons, or UI summaries. It reads the `alpha_trading` schema and
builds a backend-owned validation report. The frontend should render the report;
it should not recalculate lifecycle, accounting, or snapshot trust decisions.

## Entry Points

- `runner.rs`: public validation flow and comprehensive validation profile.
- `checks.rs`: ordered orchestration of all validation checks.
- `checks/`: focused check modules grouped by validation concern.
- `db.rs`: result-set, strategy summary, and sample-trade reads.
- `report.rs`: report, check result, verdict, and printable output types.
- `persistence.rs`: persisted validation report upsert and legacy cleanup.

The public API is `validate_strategy(pool, ValidationOptions)`.

## Validation Mode

There is one validation mode: `comprehensive`.

The validator intentionally avoids quick/strict/profile variants because a
strategy result should have one canonical trust answer for a given result set and
strategy. Persisted reports use a deterministic validation id:

```text
val_<result_set_id>_<strategy_name_or_all>_comprehensive
```

When a report is persisted, older validation rows for the same result set and
strategy are removed.

## Inputs

The validator scopes itself by:

- `result_set_id`: required.
- `strategy`: optional. When omitted, checks run across all strategies in the
  result set.

It reads these backend tables:

- `alpha_trading.backtest_result_sets`
- `alpha_trading.backtest_result_set_runs`
- `alpha_trading.trades`
- `alpha_trading.trade_events`
- `alpha_trading.trade_snapshots`
- `alpha_trading.positions`
- `alpha_trading.order_intents`
- `alpha_trading.strategy_decisions`
- `alpha_trading.risk_events`
- `risk_atlas_observations` for historical market-buy provenance checks.

## Output

`StrategyValidationReport` contains:

- result-set metadata and optional strategy filter.
- the `comprehensive` profile label.
- a summary of pass/fail/blocked counts. `warn` is retained only for backward
  compatibility with older persisted reports.
- per-strategy trade/PnL summaries.
- ordered check results.
- top-winner and worst-loser samples.
- procedure text describing the validation method.

Each `CheckResult` answers one explicit question:

- `category`: validation concern such as `accounting`, `snapshots`, or
  `decision_timing`.
- `code`: stable machine-readable check identifier.
- `question`: user-facing trust question.
- `description`: why that question matters.
- `verdict`: `pass`, `fail`, or `blocked` for current validation checks.
- `message`: concise result and violation count.
- `evidence`: structured evidence such as violation counts and tolerances.

Validation checks ask correctness questions only. Questions about whether PnL is
fragile, concentrated, or attractive belong in `strategy_assessment`.

## Procedure

The validator runs in this order:

1. Load result-set metadata, scoped strategy summaries, and trade samples.
2. Validate result-set identity and signal scope before treating PnL as usable.
3. Validate strategy decision timing and signal provenance.
4. Validate execution replay assumptions and persisted simulation outputs.
5. Validate lifecycle ordering and trade/position rollups.
6. Validate accounting from backend persisted fills and gas.
7. Validate snapshots and latest trade rollups.
8. Validate replay readiness for closed trades.

The order is intentional: basic provenance and lifecycle errors make downstream
PnL harder to interpret.

## Running

Validate and print markdown:

```bash
cargo run -p eth_alpha_lab -- strategy-validation \
  --result-set <result_set_id> \
  --strategy <strategy_name>
```

Validate and return JSON:

```bash
cargo run -p eth_alpha_lab -- strategy-validation \
  --result-set <result_set_id> \
  --strategy <strategy_name> \
  --json
```

Persist the canonical comprehensive report:

```bash
cargo run -p eth_alpha_lab -- strategy-validation \
  --result-set <result_set_id> \
  --strategy <strategy_name> \
  --persist
```

## Adding A Check

1. Put the query in the module that owns the concern under `checks/`.
2. Return a `CheckResult` using `common::count_check` or `common::check`.
3. Add the check to the ordered list in `checks.rs`.
4. Add the question and description to `checks/common.rs`.
5. Update `checks/README.md` with the code, question, and reason.
6. Run:

```bash
cargo fmt -p eth_alpha_lab --check
cargo check -p eth_alpha_lab
```

## Design Rules

- Backend persisted data is authoritative.
- The validator should never rely on frontend-derived classifications.
- Checks should report structured evidence, not just log text.
- Accounting checks should use persisted fills, gas, and snapshots.
- Snapshot checks should detect stale aggregate rollups and impossible timeline
  ordering.
- A known-bad historical result should fail in a specific, explainable way.
- Do not add strategy-quality, distribution, or profitability attractiveness
  checks here. Add those to `strategy_assessment` instead.
