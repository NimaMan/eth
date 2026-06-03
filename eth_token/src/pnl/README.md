# pnl

Pool-scoped address PnL calculation.

This module stays pure: it records token, denom, native fee, and native priority
fee movement from processed transactions and exposes address/pool summaries.
Database writes belong in `eth_token_store`.

## Transaction Ledger

`tx_ledger` is the normalization layer that should sit between
`ProcessedTransaction` evidence and pool/user PnL attribution.

Core principles:

- Capture real balance movements first. PnL should consume a movement ledger,
  not raw trace rows.
- Raw internal traces are not automatically movements. Native ETH movement
  excludes zero value, errored rows, missing recipients, `DELEGATECALL`,
  `STATICCALL`, and `CALLCODE`.
- ETH and WETH are separate representations but the same denomination family
  when the pool denom is canonical WETH. WETH deposit/withdraw events add the
  wrapped representation leg; native ETH legs still come from balance-moving
  ETH evidence.
- Reconcile before roles. Address role labels and user/router/pool attribution
  should consume per-address reconciled deltas, not decide whether evidence is a
  movement.
- Conservation is two-level: by concrete representation (`ETH`, `WETH`, token)
  and by asset family (`denom`, `token`, other assets).
- Pool PnL should consume the reconciled ledger. Pure tx-level pass-throughs are
  addresses whose asset-family deltas net to zero in that transaction.
- Router allowlists are not primary accounting logic. Known infrastructure can
  still be used as labeling or filtering context after movement reconciliation.
- Database tables should store source evidence, derived movements, and
  reconciliation summaries. They should not encode accounting decisions that
  belong in this module.

Existing tracker invariants:

- raw amounts are kept as integer strings or `U256` values;
- pool/address rollups are derived from movement rows;
- conservation checks compare total token and denom in/out movement;
- fee and native priority fee movement is tracked separately from pool denom
  cashflow;
- attribution/grouping should be layered on top of raw address rows, not baked
  into the first pass.
