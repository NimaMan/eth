# Policy Questions

These are the questions that drive the ten iterations. A policy must name which
questions it answers and which remain open.

## Entry Questions

1. Should a pool be excluded by protocol before any PnL comparison?
2. Should LP approval in the buy-confirmation block be treated as a no-entry
   proxy, an immediate de-risk signal, or a normal max-hold case?
3. Is the strategy buying before enough active observations exist to estimate
   launch momentum quality?
4. Does the entry require liquidity growth, price momentum, or both?
5. Should pools with no risk-event path be allowed, or are they outside the
   current Direct LP policy family?

## Exit Questions

1. Is mined LP approval enough to exit, or should only critical/high-confidence
   approval trigger exit?
2. Is a one-block LP approval lead actionable, or should it be treated as an
   ordering race in backtests?
3. Should liquidity removal trigger an exit, or is it already too late and only
   useful for labeling?
4. How many failed exit retries are acceptable before a position is marked as
   unrecoverable?
5. Should mark-to-market drawdown exit before LP approval appears?

## Profit Questions

1. Which winner traits are visible before the exit, not only after the fact?
2. Does a take-profit cap reduce top winners too much, or does it convert open
   exposure into reliable realized PnL?
3. Is hold length the main profit driver, or is price-to-initial acceleration
   a better exit clock?
4. Should take-profit thresholds differ for pools with same-block LP approval?
5. Are top winners concentrated enough that the policy is fragile?

## Loss Questions

1. Which losses were preventable from as-of evidence before buy submission?
2. Which losses were only visible after buy confirmation and require live
   mempool or transaction-ordering support?
3. Which losses came from LP approval lead less than or equal to one block?
4. Which losses had no pool risk event and need a non-LP scam target?
5. Which losses were mostly gas and should be handled by position sizing or
   fee caps rather than signal rules?

## Validation Questions

1. Does every compared strategy use the same range, replay run, buy size,
   liquidity floors, execution delay, and universe?
2. Does the policy use only evidence available at decision time?
3. Are same-block and next-block ordering assumptions explicitly reported?
4. Do realized and unrealized PnL reconcile with trade states?
5. Do top winners and worst losers replay cleanly under the EVM simulator?
