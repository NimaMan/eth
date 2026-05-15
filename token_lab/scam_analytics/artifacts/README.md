# Scam Analytics Artifacts

Generated files for scam analytics go here and should stay local unless they are
small, stable, and useful as regression fixtures.

Suggested layout:

```text
artifacts/
  labels/
    raw_token_server_exports/
    chain_truth_pool_events/
  features/
    per_block_snapshots/
    model_rows/
  reports/
    cohort_summaries/
```

Keep committed files human-auditable. Large JSON, CSV, parquet, notebook output,
and graph exports should be ignored or regenerated from documented commands.
