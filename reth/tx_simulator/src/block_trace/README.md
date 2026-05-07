block_trace — Block-wide Tracing

Purpose
- Replay and trace all transactions in a block using local Reth DB state, bypassing RPC.

Modules
- block_tracer/: BlockTracer engines and replay helpers.

Notes
- Fast callTracer replay uses a fused inspector between transactions.
- Arbitrary geth debug tracer options route through the Reth-style debug inspector engine.
