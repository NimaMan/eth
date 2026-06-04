# LP Approval Shared Rule

This module owns LP approval parsing and entry-gate handling shared by strategy
rules. Exit-specific policy lives in `shared_rules::exit::lp_approval`.

## Question

Was a pool's LP already meaningfully approved when the strategy was deciding
whether to enter, or did a meaningful LP approval appear after we already had an
open position?

## Evidence

The rule consumes `RiskKind::LpApproval` events scoped to the same token/pool.
When a minimum approval percentage is configured, the approval only counts when
the parsed approved percentage is strictly greater than that threshold.

The current shared default is `>30%` of LP supply. This is based on the generic
LP approval amount, not router-only approval.

## Entry Gate

If LP approval evidence is already known before a buy decision, the strategy
must not enter the pool.

This prevents buying pools where the LP owner or another privileged actor has
already made enough LP transferable to remove liquidity.

## Exit Gate

If LP approval evidence appears after entry, the exit rule decides whether to
sell the matching open position at the first valid next decision point.

This includes the important timing case where the approval appears in the same
mined block that confirms our buy. In historical replay the buy was submitted
before that approval was visible, so the entry is not an information leak, but
the newly visible approval must become an immediate de-risk signal once that
block has been processed.

The current Alpha11 policy separates launch-window approvals from later fresh
approvals. LP approvals within `<=2` chain blocks of trading enabled are treated
as no-entry/max-hold caution. Later LP approvals still trigger the normal
LP-approval exit.

## Timing Contract

Historical replay uses confirmed-chain evidence only unless a run explicitly
opts into stored mempool signals. A decision made after processing block `N`
can only use information visible at block `N`; under the current backtest fill
model, an order submitted from that decision fills in the configured later fill
block.

Live strategies can additionally act on mempool LP approval signals before they
are mined. The same shared rule should still produce the same entry/exit answer
once the signal has been converted into a `RiskKind::LpApproval` event.

Strategies must not claim an exit inside the same historical block unless the
execution model explicitly supports intra-block ordering. With the current
model, observing a mined LP approval at block `N` means submit sell after block
`N`, then fill according to the execution adapter.

## Validation Notes

Backtest validation found that many `exit.liquidity_removal` losses were pools
whose LP approval was visible in the buy-confirmation block or before the later
direct removal. These are not UI/reporting issues; they are strategy timing
cases. A correct LP-aware strategy should either skip entry when approval is
already known, or sell as soon as approval becomes known after entry.
