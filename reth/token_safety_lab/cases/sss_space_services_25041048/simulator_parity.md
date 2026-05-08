# Simulator Parity

This file tracks whether the simulator reproduces the chain behavior.

## Checks To Run

- Replay `initial_liquidity` and verify pair creation, LP state, and reserves.
- Replay `first_tracked_buy` and verify buy output and post-reserves.
- Replay `creator_sync_after_reserve_collapse` and verify whether token
  `balanceOf(pool)` collapses before `sync()`.
- Replay `weth_drain_sell` and verify the observed sell-like tx succeeds and
  drains WETH.
- Replay `later_buy` and verify the post-buy reserve state.

## Failure Classification

If simulator output differs from chain truth, classify the mismatch as one of:

- missing prior tx setup;
- missing token control or balance mutation;
- incorrect pool/token metadata;
- incorrect pool orientation;
- incorrect tax or sell simulation path;
- simulator EVM/state bug;
- expected limitation with a documented reason.

## Current Status

Pending. The case has chain observations, but transaction-level simulator parity
still needs to be run.
