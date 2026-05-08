# Chain Truth Tools

These tools should extract the facts from chain data:

- receipts and logs for key transactions;
- pool `Sync`, `Swap`, `Mint`, and `Burn` events;
- ERC-20 transfers and approvals;
- `getReserves()` at selected blocks;
- `balanceOf(pool)` for token and denom before and after key transactions;
- caller, input selector, value, status, and gas data.

The output of these tools is the baseline. Simulator output must be compared
against this before we trust a token or pool number.
