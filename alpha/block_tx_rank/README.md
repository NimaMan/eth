# block_tx_rank

Crate: `eth_block_tx_rank`

Rough transaction position estimates from recently mined blocks.

## Purpose

- Compare a candidate EIP-1559 transaction against effective priority fees from
  recent mined transactions.
- Estimate how many mined transactions would have ranked ahead of the candidate
  and how much gas would likely be consumed before it.
- Return rank evidence that the live trading planner can persist with the order
  decision before calling `tx_executor`.

## Owns

- Candidate gas profile types used for rank checks.
- Recent mined block fee samples.
- Rough position, gas-before, and fee-percentile bands.
- Block-rank evidence for strategy and risk decisions.

## Does Not Own

- Pending mempool ingestion or pending transaction simulation.
- Route construction, quotes, slippage, or AMM policy.
- Signing, nonce reservation, or broadcast.
- Final strategy approval.

## Integration Shape

```text
alpha/live/trading GasRankProvider
  -> build candidate gas profile
  -> eth_block_tx_rank estimates recent mined-block rank
  -> tx_prep accepts, rejects, or adjusts the gas plan under the value cap
  -> tx_executor receives only the final prepared transaction
```

`tx_executor` should stay a submission boundary. If a strategy needs block
position awareness, it should consume this crate before constructing the final
`DirectRawTransactionRequest`.

## Current Integration Gap

The crate is not yet on the live strategy critical path. `alpha/live/trading`
accepts `RankedFeeCandidate`s and can reject priority exits whose required bribe
exceeds protected value, but no live adapter currently queries this crate and
passes those candidates into `tx_prep`.

For first deployment, the production `GasRankProvider` should call this crate
before Kartal submission and persist the selected rank evidence in request
metadata.

## Bribe Candidate Discussion

The live planner should use this crate to ask a narrow question:

```text
Given this route gas estimate and next-block base-fee forecast, what priority
fee bands would likely place us early enough in a recent mined block?
```

The answer should be a small list of `RankedFeeCandidate`s, not a single blind
number. Useful candidate labels are operational bands such as:

- `p50_top_10`: priority fee that would have ranked around the first 10 txs in
  the recent sample.
- `p50_top_25`: less expensive but still early.
- `p75_top_10`: higher-percentile band for pre-mine liquidity-removal races.

`alpha/live/trading::tx_prep` remains the final decider. It filters candidates
against protected value:

```text
protected_exit_value
  - expected_late_recovery
  - safety_buffer
  - estimated_base_fee_cost
  = max_priority_spend
```

Candidates whose priority spend or total max-fee spend exceed that cap are
rejected. Among the remaining candidates, the planner prefers the best expected
rank, then the higher priority fee when ranks tie. If every ranked candidate is
too expensive, public mempool submission should reject rather than leak a weak
transaction. The planner should not create a synthetic value-cap candidate when
rank evidence is missing or too expensive.

The metadata handed to Kartal should include the candidate label, source sample
window, predicted base fee, priority fee, max fee, rank estimate, gas-before
estimate, and whether the candidate was selected or rejected by the value cap.

## Tests

```bash
cargo test -p eth_block_tx_rank
```
