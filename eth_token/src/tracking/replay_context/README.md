# Simulation Triggers

`replay_context` now only owns token-control trigger policy used by token/pool
simulation.

Historical token processing uses `BlockTxStateSession` to branch from the
canonical state after the current mined transaction. Live token processing uses
`BlockStateSession` to branch from the selected live/persisted block state.

The low-level `PoolBuySellParameters.prior_txs` API remains in `tx_processor`
for direct and mempool simulations, but token tracking no longer builds
same-block prior replay lists.
