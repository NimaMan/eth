# Token Lab

Concrete token and pool investigations for Risk Atlas live here. This folder is
for reproducible case evidence: what a token/pool did on chain, what the source
read models recorded, whether the simulator can reproduce the route, and what
detector, display, or Alpha input should follow.

Token Lab is intentionally narrower than `../investigations/`:

- `cases/` owns concrete token/pool case notes.
- `categories/` owns the token-lab case taxonomy and per-category indexes.
- `../investigations/parity/` owns the shared chain/source/simulator parity
  method.
- `../investigations/behavior_catalog/` owns shared mechanism labels learned
  across cases.
- `../../alpha/lab/strategy_analysis/` owns strategy-policy interpretation of
  validated facts.

## Layout

| Path | Purpose |
| --- | --- |
| `cases/` | Concrete token/pool investigations with narrative, metadata, and artifacts. |
| `categories/` | Case categories used by `investigation.toml` and the Risk Atlas promotion queue. |

Generated artifacts belong under each case's `artifacts/` folder and should not
be promoted into trading or UI behavior until the durable fact is represented in
the owning module.
