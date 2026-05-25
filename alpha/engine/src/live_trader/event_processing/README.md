# Event Processing

The live trader processes one loop tick in this order:

1. Read chain-server live status, latest pool surface, and mempool signals.
2. Publish every fetched pool snapshot into the execution adapter pool cache.
3. Settle already-submitted executions:
   - chain-sim live backtests simulate due submissions against the exact target
     block state;
   - real trading reconciles Kartal submissions from mined receipts and vault
     events.
4. Process eligible mempool risk signals.
5. Process changed pool updates as `MarketEvent::PoolUpdated`.
6. Process mined pool-risk candidates discovered from those pool updates.
7. Process manual close requests for real trading.
8. Run the position monitor as `MarketEvent::BlockCompleted`.
9. Write heartbeat and health metadata.

The important rule is that settlement for already-submitted executions happens
before strategies see the next block's market/risk events. That mirrors real
trading: a transaction submitted at block `N` is observed in later mined block
evidence before the strategy starts making new decisions from that later block.

Strategy observations are still recorded at their source event:

- pool decisions are persisted against the pool update block;
- mempool risk decisions are persisted against the signal observed block;
- position-monitor decisions are persisted against the completed block;
- execution settlement is persisted as an execution event with the report block.

For chain-sim live backtests, a submitted report has
`mined_evidence.receipt_status = live_backtest_chain_sim_submitted`. A final
settlement report has `receipt_status = live_backtest_chain_sim`,
`submitted_block_number`, `expected_confirmation_block`,
`simulation_block_number`, and `confirmation_lag_blocks`.
