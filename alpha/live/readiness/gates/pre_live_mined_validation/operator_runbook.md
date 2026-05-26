# Operator Runbook

Run this sequence for a one-position mined-validation attempt.

1. Confirm the strategy-specific readiness file names the exact validation
   strategy and expected caps.
2. Confirm no conflicting live trader process is running.
3. Confirm Kartal is healthy and in `dry_run`.
4. Confirm the signer is healthy.
5. Confirm executor and signer target/selector allowlists match the deployed
   vault route.
6. Confirm value, gas, fee, and transaction-cost caps cover the validation buy
   and sell; confirm Alpha's strategy bankroll is the active budget limiter.
7. Run dry-run calibration and archive the report.
8. Run one final dry-run signing check after cap changes.
9. Switch only Kartal broadcast mode to `broadcast`.
10. Start the validation strategy by name with the explicit broadcast
    validation guard.
11. Watch for either a complete buy/sell lifecycle or a documented failure.
12. Stop the validation run.
13. Collect the required evidence in `evidence.md`.
14. Decide whether the gate passed, failed safely, or needs another attempt.
15. Return Kartal to `dry_run` unless the next approved step explicitly needs
    public broadcast.

The runbook deliberately separates strategy selection from runtime parameters:
live strategy parameters must come from the named strategy spec.
