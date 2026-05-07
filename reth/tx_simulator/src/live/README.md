# Live Simulation

The `live` module is the entry point for latency-sensitive pipelines where the
target state can be ahead of the locally persisted Reth MDBX database.

`LiveTxSimulator` chooses state in this order:

1. Redis chain-state overlay written by the live block processor.
2. Latest persisted MDBX block when no live overlay is available.

Regular `TxSimulator` APIs remain the lower-level historical/direct-DB surface.
Live pipelines should depend on `tx_simulator::live::LiveTxSimulator` so latest
state selection stays explicit.
