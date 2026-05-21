# Pre-Live Mined Validation Gate

Every strategy must pass a one-position mined-validation gate before it can use
public real capital. The gate is not a profitability test. It proves the
chain-related assumptions that live backtests cannot prove.

The reusable gate is split into:

- `checklist.md`: pass/fail checks.
- `evidence.md`: required artifacts.
- `failure_modes.md`: what this gate does and does not prove.
- `operator_runbook.md`: execution sequence.

For Alpha11, the current concrete instance is:

`../../strategies/alpha11/hold3_mined_validation.md`

