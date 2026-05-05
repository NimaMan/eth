# chain_state

Encoded chain-state overlays.

`LiveTxSimulator` currently serializes a REVM cache overlay for the latest processed blocks. This module keeps the live-state contract aware of those snapshots without making the protocol crate depend on the simulator or REVM cache internals.

`EncodedChainStateSnapshot` is the typed store API envelope. A Redis backend may still store only the codec-specific `payload` bytes at `eth/live/block/<n>/chain_state_snapshot` to preserve compatibility with the current simulator while deriving metadata from the decoded value.
