# Token Safety Lab

This folder is for repeatable investigations of token and pool behavior that may
affect trading safety.

The workflow is intentionally iterative:

1. Run the token range builder over a block range.
2. Find odd behavior, unsafe behavior, or suspicious numerical output.
3. Create a case with the chain truth and simulator parity checks.
4. Fix the indexer, simulator, or display logic when it disagrees with chain behavior.
5. Promote confirmed patterns into detectors and trading guardrails.
6. Run the range again and repeat.

The objective is to trust the first numbers we see. A number is trusted only
after it either matches chain behavior or the mismatch is explained and tracked.

## Folder Layout

- `cases/`: one folder per concrete token or pool investigation.
- `odd_behaviors/`: the shared catalog of suspicious patterns we want to detect.
- `tools/chain_truth/`: tools that extract on-chain receipts, logs, balances, and reserves.
- `tools/parity/`: tools that compare simulator replay against chain truth.
- `tools/trading/`: tools that compare observed on-chain buys and sells against our simulated trades.
- `tools/detectors/`: prototype detectors before promotion into production code.

## Case Standard

Every case should include:

- `case.toml`: token, pool, block range, and key transactions.
- `chain_truth.md`: what happened on chain, with block and tx references.
- `simulator_parity.md`: what our simulator reproduced and what it failed to reproduce.
- `findings.md`: confirmed lessons, bugs, detector candidates, and follow-up fixes.
- `artifacts/`: generated outputs, receipts, traces, and comparison files.
