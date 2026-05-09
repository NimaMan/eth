# Tools

Tools in this folder are investigation tools. Production code should not depend
on them directly until a pattern is promoted into a crate-level detector,
simulator fix, or regression test.

Tool groups:

- `chain_truth/`: extract chain data.
- `parity/`: compare simulator replay against chain data.
- `trading/`: compare observed trades against simulated trades.
- `detectors/`: prototype odd-behavior detectors.
