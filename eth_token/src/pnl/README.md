# pnl

Pool-scoped address PnL calculation.

This module stays pure: it records token, denom, native fee, and bribe movement
from processed transactions and exposes address/pool summaries. Database writes
belong in `eth_token_pnl_store`.

Core invariants:

- raw amounts are kept as integer strings or `U256` values;
- pool/address rollups are derived from movement rows;
- conservation checks compare total token and denom in/out movement;
- fee and bribe movement is tracked separately from pool denom cashflow;
- attribution/grouping should be layered on top of raw address rows, not baked
  into the first pass.
