# Event Processing

The live trader processes one loop tick in this order:

1. Read the next chain-server live block frame and mempool signals. The block
   frame comes from `/api/v1/eth/live-trading/block-frames/next`, which waits
   for or returns the next retained frame after Alpha's last processed block.
2. Publish every block-frame pool snapshot into the execution adapter pool
   cache.
3. Settle already-submitted executions:
   - chain-sim live backtests simulate due submissions against the exact target
     block state;
   - real trading reconciles Kartal submissions from mined receipts and vault
     events.
4. If chain-sim settlement is due but the exact confirmation-block state is not
   ready yet, buffer fetched mempool signals and stop this tick before strategy
   event processing. Those buffered signals are processed on the next tick after
   settlement succeeds.
5. Process changed pool updates as `MarketEvent::PoolUpdated`.
6. Process mined pool-risk candidates discovered from those pool updates.
7. Process eligible mempool risk signals using the signal's
   `detected_at_head_block_number` as the risk observed block.
8. Process manual close requests for real trading.
9. Run the position monitor as `MarketEvent::BlockCompleted`.
10. Write heartbeat and health metadata.

The important rule is that settlement for already-submitted executions happens
before strategies see the next block's market/risk events. That mirrors real
trading: a transaction submitted at block `N` is observed in later mined block
evidence before the strategy starts making new decisions from that later block.
In chain-sim live backtests this is a hard gate: if the exact block state for a
due simulated confirmation is not available yet, mempool signals are retained
but no risk, pool, mined-risk, or position-monitor strategy decision is emitted
from the stale portfolio state.

Strategy observations are still recorded at their source event:

- pool decisions are persisted against the pool update block;
- mempool risk decisions are persisted against the detector-time head block
  carried by the signal;
- position-monitor decisions are persisted as `pool_update` observations with a
  `position_monitor:<block>` key against the completed block;
- execution settlement is persisted as an execution event with the report block.

## Event Labels

These labels are the canonical live-trader labels. The source label must say
where the evidence came from, not what the strategy decided to do with it.
Pending public mempool evidence uses `mempool_signal`. Confirmed mined evidence
from the live chain server, including mined LP approvals and liquidity removals,
uses `pool_update`.

| Evidence path | Persisted source label | Risk kind / event | Strategy reason | Gas-policy action |
| --- | --- | --- | --- | --- |
| Live chain-server pool snapshot or mined pool-risk field | `pool_update` | market pool update, `lp_approval`, `liquidity_removal`, completed-block maintenance | `entry.buy_eligible_pool_once`, `exit.lp_approval`, `exit.liquidity_removal`, `exit.max_hold_active_blocks` | `entry_buy`, `lp_approval_exit`, `mined_liquidity_removal_exit`, `normal_exit` |
| Pending public mempool signal | `mempool_signal` | `lp_approval`, `mempool_liquidity_removal`, `trading_enabled` | `exit.lp_approval`, `exit.mempool_liquidity_removal_signal`, `entry.tail_after_enabling_tx` | `mempool_race_exit` for exits, `tail_entry_buy` for tail entries |
| Manual operator close | `manual_close` | manual close request | `manual.close_position` | `normal_exit` |

Chain-sim settlement unavailability is not used to defer unrelated mempool
signals. If chain-server cannot serve exact state for a submitted order's
execution block, that submitted order remains pending and the next live tick
continues processing new strategy events from the next retained block frame.

The live tick is block-pinned. Chain-server has already updated its
`LiveTxSimulator`, token state, and pool state before the frame is visible to
Alpha. Alpha real mode may turn strategy decisions into Kartal submissions;
chain-sim mode asks chain-server to simulate submitted orders at their exact
target block.

For chain-sim live backtests, a submitted report has
`mined_evidence.receipt_status = live_backtest_chain_sim_submitted`. A final
settlement report has `receipt_status = live_backtest_chain_sim`,
`submitted_block_number`, `expected_confirmation_block`,
`simulation_block_number`, `confirmation_lag_blocks`, and the chain-server
simulation block hash when available.
