# Iteration 06 - Clean-Lead LP Exit

Question:

```text
Should mined LP approval exits only receive clean historical credit when the
approval lead is greater than one block?
```

Policy change:

- Split LP approval exits into:
  - clean lead: removal occurs two or more blocks later;
  - race lead: removal occurs in the next block;
  - too late: removal already mined.

Reason:

- 58 eligible direct-LP scams had a one-block warning window.
- Loss scan found 21 losing trades with LP approval lead less than or equal to
  one block and 19 sell confirmations racing liquidity removal.

Implementation status:

- Current backtest records the evidence needed for review.
- A new coded variant is needed if clean-lead gating should change decisions
  during replay instead of only annotating results.

Review gate:

- Report PnL separately for clean-lead, one-block race, and too-late cases.
- Do not count one-block exits as clean strategy edge.
