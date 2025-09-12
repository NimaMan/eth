block_trace — Block-wide Tracing

Purpose
- Replay and trace all transactions in a block using local Reth DB state, bypassing RPC.

Modules
- block_simulation/: BlockTracer and related types (moved from block_simulation/).

Notes
- Mirrors Reth debug block tracing (construct env from canonical headers; fuse inspector between txs).
- Exports geth-compatible frames for equivalence with debug_traceBlockByNumber.

