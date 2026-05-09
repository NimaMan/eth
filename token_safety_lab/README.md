# Token Safety Lab

This folder is for repeatable investigations of token and pool behavior that may
affect trading safety.

The workflow is intentionally iterative:

1. Run the token range builder over a block range.
2. Find odd behavior, unsafe behavior, or suspicious numerical output.
3. Create a case with the chain truth and simulator parity checks.
4. Fix the indexer, simulator, or display logic when it disagrees with chain behavior.
5. Promote confirmed patterns into detectors and trading guardrails.
6. Run the range again and repeat.

The objective is to trust the first numbers we see. A number is trusted only
after it either matches chain behavior or the mismatch is explained and tracked.

## Triage First

Before deep case work, run a triage pass over the active range-builder output
and simulator logs. The first artifact should be a candidate ledger, not a fix.
Use the candidate ledger in `cases/README.md` for the working list.

Use the agent-native range triage tool to turn a completed range run into
structured candidates. Run commands from the ETH repo root:

```text
token_safety_lab/tools/detectors/range_triage.py \
  --api http://127.0.0.1:8765 \
  --run active \
  --format markdown
```

The tool is read-only. It inspects the token server run cache and emits
candidate issues for review; it does not create cases or change token state.

Each candidate row should capture:

- token address and symbol
- pool address, when the issue is pool-specific
- block range and important block or tx hash
- visible symptom
- why it matters for trading safety
- current status: `new`, `investigating`, `confirmed`, `explained`, `fixed`, or `ignored`
- linked case folder once promoted

Candidate buckets to collect:

- Huge `price / initial` ratios, especially when liquidity is low or token
  supply in pool is tiny.
- `can_buy=yes` with `can_sell=no`.
- Scam or risk rows, especially `CANNOT_SELL`, liquidity drain, denom removal,
  hidden mint, high tax, and trading-state flips.
- Tax buckets from `eth_token::pools::TaxBucket`: `no_tax`, `low_tax`
  (`0 < tax < 10%`), `moderate_tax` (`10-20%`), `high_tax` (`20-40%`),
  and `extreme_tax` (`>40%`).
- Tokens without pools that appear to have trading enabled or other pool-derived
  state.
- Impossible or suspicious numbers: supply in pool over 100%, LP holder shares
  over 100%, zero LP supply with reserves, liquidity/FDV near zero with
  meaningful WETH, or FDV driven by dust reserves.
- Simulator warnings and errors from
  `/home/nima/code/crypto/blockchains/eth/logs/simulators/`.
- Chain-vs-simulator mismatches, including cases where on-chain sells exist but
  the simulator cannot reproduce a sell at the same pre-state.

Promote a candidate into `cases/<slug>/` when it affects trading decisions,
suggests a pipeline bug, or should become a detector or guardrail. Do not write
safety-lab artifacts from normal range builds; case tools should write generated
outputs under the case `artifacts/` folder.

## Folder Layout

- `cases/`: one folder per concrete token or pool investigation.
- `odd_behaviors/`: the shared catalog of suspicious patterns we want to detect.
- `tools/chain_truth/`: tools that extract on-chain receipts, logs, balances, and reserves.
- `tools/parity/`: tools that compare simulator replay against chain truth.
- `tools/trading/`: tools that compare observed on-chain buys and sells against our simulated trades.
- `tools/detectors/`: prototype detectors before promotion into production code.

## Case Standard

Every case should include:

- `case.toml`: token, pool, block range, and key transactions.
- `chain_truth.md`: what happened on chain, with block and tx references.
- `simulator_parity.md`: what our simulator reproduced and what it failed to reproduce.
- `findings.md`: confirmed lessons, bugs, detector candidates, and follow-up fixes.
- `artifacts/`: generated outputs, receipts, traces, and comparison files.
