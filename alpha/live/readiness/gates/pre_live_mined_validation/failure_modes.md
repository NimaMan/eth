# Failure Modes

The one-position mined-validation gate proves the happy path for a single buy
and sell. It does not prove the full production envelope.

## Covered

- Final deployed-vault calldata simulation.
- Kartal policy acceptance.
- Signer policy acceptance.
- Signing.
- Public mempool broadcast.
- Buy receipt reconciliation.
- Sell trigger after the short hold.
- Sell receipt reconciliation.
- Receipt-derived gas and amount accounting.

## Not Covered

- Nonce replacement.
- Dropped transactions.
- Reorgs.
- Delayed inclusion.
- Base-fee or priority-fee spikes.
- Revert after a previously passing simulation.
- Stale local Reth simulation state under high lag.
- Multiple concurrent open positions.
- Sell retry policy under urgent risk events.
- RPC outage during receipt polling.
- Receipt finality beyond the configured recheck depth.

After this gate passes, the next readiness step is a capped multi-position
validation run before increasing bankroll or deploying the main strategy.

