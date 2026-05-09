# Trading Tools

Trading tools answer the question that matters for execution:

Can we safely buy and sell this token from the same pre-state that existed on
chain?

Required checks:

- simulate buy at the target block;
- simulate sell after the buy;
- compare with observed on-chain buys and sells;
- classify failures as token behavior, router behavior, state setup, or simulator bug;
- record tax and received amounts in denom units.

Observed on-chain sells are especially important. If the chain has a successful
sell but our simulator says selling is impossible, we must investigate before
using that simulator result as a trading guardrail.
