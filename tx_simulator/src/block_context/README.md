# Block Context

Shared logic for loading the header and state needed to simulate a transaction
or block.

## Responsibilities

1. Load canonical headers from the local Reth provider.
2. Load historical state from local Reth when the requested block is available.
3. Build direct live block sessions from caller-supplied headers plus
   `prestateTracer` diffMode output.

Live processors that already have block headers and state diffs should pass
those values directly to `block_state_session_from_prestate_diffs` or
`block_state_session_from_parent_prestate_diffs`.
