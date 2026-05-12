# Pipeline Assessment Plan

This folder will host quick scripts/notebooks for validating the signal pipeline end-to-end. The goals are:

1. **Cross-check per-stage outputs**
- Compare `function_detector/liquidity_removals.log` vs `signals/liquidity_removals.log` to ensure every function-level detection eventually emits a signal (or understand gaps).
- Pair `function_detector/trading_enabled.log` entries with `signals/trading_enabled.log`.
  - Remember: the function detector logs *every* mempool transaction that matches a selector, while the signals directory captures the subset (primarily creator transactions) that survive simulation and cache checks.

2. **Simulation error validation**
   - Parse run-level `simulation_errors.log` and group failures by error class,
     token, pool, and function type.
   - Confirm actionable errors are reflected in service logs
     (`signal_detector.log`) and do not prevent unrelated pool signals from
     being emitted.
   - Successful simulations are counted in interval metrics, not written
     one-by-one.

3. **Signal completeness checks**
   - Count per-signal totals in the log files and compare with ZMQ archive / DB entries (where available).
   - Detect duplicate emissions for the same `(token, pool, tx_hash)` tuple.

4. **Automation roadmap**
   - Start with small Python scripts that read a run directory, aggregate metrics, and emit a short report (e.g., JSON or Markdown summary).
   - Later extend to pytest-based regression tests that can be run after large refactors.

## Next Steps

- Implement a helper (`analyze_run.py`) that takes a run directory path and prints basic counts and mismatches.
- Add optional flags to compare against Postgres records (since signals are persisted by default).
- Document expected log schemas (per line formats) so we can parse reliably.
