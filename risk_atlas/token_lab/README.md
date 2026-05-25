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

## Case Questions

Every case should make the answer to these questions explicit:

| Question | Why It Matters |
| --- | --- |
| What pool, token, block range, and transactions anchor the case? | Keeps the case reproducible. |
| What was the first suspicious or decision-relevant chain event? | Separates cause from later symptoms. |
| What did source observations and token-builder state report at the same coordinates? | Checks whether Risk Atlas inputs match chain truth. |
| Can the simulator and Alpha executable route reproduce the relevant buy/sell state? | Determines whether the fact can support strategy or live execution decisions. |
| Which category owns the case: parity, mechanism, display, or execution viability? | Prevents one case from becoming a catch-all note. |
| What durable label, display rule, model feature, or Alpha input should follow? | Defines the promotion path. |
| What is explicitly out of scope? | Keeps position/PnL validation in Alpha Lab and network/fund-flow work in Network Analytics. |

Generated artifacts belong under each case's `artifacts/` folder and should not
be promoted into trading or UI behavior until the durable fact is represented in
the owning module.
