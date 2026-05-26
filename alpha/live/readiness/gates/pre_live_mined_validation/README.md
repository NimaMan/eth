# Pre-Live Mined Validation Gate

Every strategy must pass a one-position mined-validation gate before it can use
public real capital. The gate is not a profitability test. It proves the
chain-related assumptions that live backtests cannot prove.

The reusable gate is split into:

- `checklist.md`: pass/fail checks.
- `evidence.md`: required artifacts.
- `failure_modes.md`: what this gate does and does not prove.
- `operator_runbook.md`: execution sequence.

Strategy-specific evidence should link back to this gate from the current
strategy readiness file. Alpha11's current deploy readiness is tracked in
`../../strategies/alpha11/hold16_deploy_readiness.md`.
