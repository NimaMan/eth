# Findings

## Confirmed So Far

- The final pool reserves match chain replay.
- The extreme price ratio comes from a tiny token reserve against non-zero WETH.
- The pool has at least one observed sell-like transaction on chain.
- The creator-triggered `sync()` after token reserve collapse is a critical
  suspicious event.

## Detector Candidates

- `sync_without_transfer`
- `reserve_discontinuity`
- `token_reserve_dust`
- `price_ratio_extreme_low_supply`
- `observed_sell_simulator_fail`

## Open Work

- Confirm simulator replay parity for the key transactions.
- Determine whether the observed sell can be reproduced by the simulator at the
  same pre-state.
- Decide whether the range builder should flag this pool before the WETH drain,
  at the direct `sync()`, or both.
- Decide how Asena should display price ratio when reserve quality is unsafe.
