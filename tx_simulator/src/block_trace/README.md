block_trace — Block-wide Tracing

Purpose
- Replay and trace all transactions in a block using local Reth DB state, bypassing RPC.

Modules
- block_tracer/: BlockTracer engines and replay helpers.
- ../session/block_replay.rs: `BlockReplaySession`, a pinned block replay/profile builder.

Notes
- Fast callTracer replay reuses one inspector and resets its per-tx trace buffers between transactions. Upstream Reth/REVM names that reset `fuse`.
- Arbitrary geth debug tracer options route through the Reth-style debug inspector engine.
- `execute_only_profile` is a no-trace lower-bound diagnostic; full-trace acceptance should use
  `trace` or `profile`.
