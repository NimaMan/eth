# Investigations

Cross-cutting Risk Atlas investigation material lives here. This folder is for
methods and shared knowledge that apply across many token/pool cases.

Concrete token and pool case writeups live in `../token_lab/cases/`; the case
taxonomy lives in `../token_lab/categories/`.

## Layout

| Path | Purpose |
| --- | --- |
| `methodology/` | Evidence contract, status terms, and promotion rules for investigations. |
| `parity/` | Chain truth vs token-builder/source observation vs simulator vs Alpha route rules. |
| `behavior_catalog/` | Shared suspicious-behavior and scam-mechanism labels learned across cases. |
| `promotion_queue/` | Ranked cross-case queue for unresolved or promotion-blocking questions. |
| `../token_lab/cases/` | Concrete token/pool investigations with narrative, metadata, and artifacts. |
| `../token_lab/categories/` | Token-lab case categories and per-category case indexes. |

Investigation scripts belong in the module that owns the capability. Token
tracking audits belong under `eth_token`, route replays belong under
`tx_simulator` or `tx_processor`, and strategy analysis belongs under
`alpha/lab`.

## Boundary

Use `token_lab/cases/` when the evidence is about a specific token, pool, block
range, strategy position, or transaction sequence.

Use this folder when the evidence becomes a reusable method, a shared parity
rule, a mechanism label, or a cross-case promotion blocker.
