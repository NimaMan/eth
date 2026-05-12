# block_tx_rank

Crate: `eth_block_tx_rank`

Rough transaction position estimates from recently mined blocks.

## Purpose

- Compare a candidate EIP-1559 transaction against effective priority fees from
  recent mined transactions.
- Estimate how many mined transactions would have ranked ahead of the candidate
  and how much gas would likely be consumed before it.
- Return rank evidence that a future live trading adapter can persist with the
  order decision before calling `tx_executor`.

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
strategy decision / real execution adapter
  -> build candidate gas profile
  -> eth_block_tx_rank estimates recent mined-block rank
  -> strategy/risk accepts, rejects, or adjusts the order
  -> tx_executor receives only the final prepared transaction
```

`tx_executor` should stay a submission boundary. If a strategy needs block
position awareness, it should consume this crate before constructing the final
`DirectRawTransactionRequest`.

## Tests

```bash
cargo test -p eth_block_tx_rank
```
