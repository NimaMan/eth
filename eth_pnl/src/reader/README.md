# Token PnL Reader

This module owns read-only views over persisted `token_pnl` data.

- `run.rs` selects and serializes calculation run metadata.
- `sql.rs` builds shared address aggregate SQL and route-specific queries.
- `rows.rs` maps SQL rows into stable HTTP JSON fragments.
- `address_pnl.rs` assembles list, profile, and trade-detail payloads.

Risk Atlas may import the shared run selector and aggregate CTE for analytics,
but route payload assembly for `/api/v1/eth/addresses*` lives here.
