# Token Lab

Agent operating map for repeatable token, pool, launch, market-behavior, and
trading-strategy investigations.

## Purpose

- Turn token/pool behavior into reproducible cases and reusable analysis.
- Compare range-builder output, chain truth, simulator replay, and trading
  guardrail expectations.
- Promote confirmed patterns into production detectors, frontend summaries,
  strategy analytics, or alpha guardrails.

## Owns

- Case folders with the narrative, machine-readable facts, and generated
  artifacts for one concrete investigation.
- Strategy analysis notes and contracts for launch/winner/scam cohort stats.
- Read-only triage and detector prototype tools.
- Chain-truth, parity, and trading comparison scripts.
- Shared catalog of odd behavior patterns.

## Does Not Own

- Production token state logic; fix confirmed state bugs in `eth_token`.
- Simulation internals; fix replay mismatches in `tx_simulator` or
  `tx_processor`.
- Live mempool signal emission; promote detectors into `mempool_processor`.
- Strategy policy; promote guardrails into `alpha`.

## Data Flow

```text
token range/server output + simulator logs
  -> read-only triage ledger
  -> case folder with chain truth and parity artifacts
  -> confirmed finding
  -> fix in owner crate or promote detector/guardrail
```

## Folder Structure

| Folder | Purpose |
| --- | --- |
| `cases/` | Concrete token or pool investigations with narrative, metadata, and artifacts. |
| `odd_behaviors/` | Shared catalog of behavior patterns that deserve review or detectors. |
| `strategy/` | Launch, winner, scam/risk, and cohort analysis used to design trading strategies. |
| `tools/chain_truth/` | Chain-fact extraction used as the baseline for trust. |
| `tools/parity/` | Simulator-vs-chain replay checks. |
| `tools/trading/` | Buy/sell and route behavior checks for execution viability. |
| `tools/detectors/` | Agent-native prototype detectors over completed range/live data. |

## Where To Look First

| Need | Start here |
| --- | --- |
| Working candidate ledger | `cases/README.md` |
| One concrete investigation | `cases/<slug>/README.md` |
| Case metadata shape | `cases/<slug>/case.toml` |
| Launch/winner strategy stats | `strategy/README.md` |
| Triage candidate generation | `tools/detectors/README.md`, `tools/detectors/range_triage.py` |
| Receipts/logs/balances/reserves truth | `tools/chain_truth/` |
| Simulator parity checks | `tools/parity/` |
| Observed trading comparison | `tools/trading/` |
| Shared suspicious patterns | `odd_behaviors/README.md` |

## Tests And Commands

Run from the ETH repo root:

```bash
token_lab/tools/detectors/range_triage.py \
  --api http://127.0.0.1:8765 \
  --run active \
  --format markdown
```

## Current Hazards

- The first artifact should be a candidate ledger, not a fix.
- Do not write generated token-lab artifacts from normal range builds. Case
  tools should write under `cases/<slug>/artifacts/`.
- Promote a case only when it affects trading decisions, suggests a pipeline
  bug, or should become a detector/guardrail.
- A number is trusted only after it matches chain behavior or the mismatch is
  explained and tracked.
