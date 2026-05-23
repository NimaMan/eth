# Gamma 10 Policy Questions

These are the questions gamma_10 is designed to answer. Each strategy maps to
one or more of them.

## Entry And Universe

1. Is the Direct LP edge still primarily a UNISWAP-V2 edge in the current 50K
   Risk Atlas run?
2. Should same-confirmation-block LP approval be treated as too dangerous, or
   is it the launch cohort that pays for the strategy?
3. Are any submit-visible features strong enough to block bad entries, or are
   they flat across winners and losers?
4. Does the policy need live ordering/mempool support before it can safely
   trade the same-confirm LP approval branch?

## Exit Timing

1. Is hold15 too late for pools whose direct LP removal happens within 15 blocks
   after entry?
2. Does hold5, hold8, hold10, or hold12 preserve most winner PnL while reducing
   removal-before-exit losses?
3. Did alpha_10 longer holds underperform because they disabled buy-confirm LP
   approval deferral?
4. Do exit retries add real economic value, or only small dust-level recovery?
5. Is confirmed liquidity removal an actionable exit signal, or mainly a loss
   label after execution has already lost the race?

## Loss Prevention

1. Can mark-to-market drawdown prevent direct LP losses before approval/removal
   exits fire?
2. Are one-block LP approval leads too optimistic under the current next-block
   execution model?
3. Which losses had no pool risk event, and are they small enough to leave out
   of the Direct LP policy family for now?
4. Are gas-heavy losses a signal problem or an execution sizing/fee-cap problem?

## Profit Improvement

1. Does a shorter hold window materially reduce the top-winner set?
2. Does take-profit at 5x realize enough upside, or does it cut the launches
   that fund the losses?
3. Can stop-loss at 0.85 reduce the loss bucket without chopping noisy winners?
4. Is top-winner concentration still low enough after each gamma policy?

## Promotion Standard

A gamma policy should not replace the alpha_10 leader unless it:

1. improves total and realized PnL on the same 50K result set;
2. does not increase open or failed exposure to hide losses;
3. reduces the direct-removal race loss bucket or explicitly explains why it
   cannot be reduced from chain-only evidence;
4. preserves no-leakage timing under the next-block execution model;
5. remains profitable after checking top-winner concentration and worst losses.
