# Token Safety Lab

Agent operating map for repeatable token and pool behavior investigations that
may affect trading safety.

## Purpose

- Turn suspicious token/pool behavior into reproducible cases.
- Compare range-builder output, chain truth, simulator replay, and trading
  guardrail expectations.
- Promote confirmed patterns into production detectors or strategy safeguards.

## Owns

- Case folders with the narrative, machine-readable facts, and generated
  artifacts for one concrete investigation.
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

## Where To Look First

| Need | Start here |
| --- | --- |
| Working candidate ledger | `cases/README.md` |
| One concrete investigation | `cases/<slug>/README.md` |
| Case metadata shape | `cases/<slug>/case.toml` |
| Triage candidate generation | `tools/detectors/README.md`, `tools/detectors/range_triage.py` |
| Receipts/logs/balances/reserves truth | `tools/chain_truth/` |
| Simulator parity checks | `tools/parity/` |
| Observed trading comparison | `tools/trading/` |
| Shared suspicious patterns | `odd_behaviors/README.md` |

## Tests And Commands

Run from the ETH repo root:

```bash
token_safety_lab/tools/detectors/range_triage.py \
  --api http://127.0.0.1:8765 \
  --run active \
  --format markdown
```

## Current Hazards

- The first artifact should be a candidate ledger, not a fix.
- Do not write generated safety-lab artifacts from normal range builds. Case
  tools should write under `cases/<slug>/artifacts/`.
- Promote a case only when it affects trading decisions, suggests a pipeline
  bug, or should become a detector/guardrail.
- A number is trusted only after it matches chain behavior or the mismatch is
  explained and tracked.
