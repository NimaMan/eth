# chain_state

Encoded chain-state overlays.

`LiveTxSimulator` currently serializes a REVM cache overlay for the latest processed blocks. This module keeps the live-state contract aware of those snapshots without making the protocol crate depend on the simulator or REVM cache internals.

`EncodedChainStateSnapshot` is the typed store API envelope. The payload stays
codec-specific so this crate can version the contract without depending on
simulator internals.
