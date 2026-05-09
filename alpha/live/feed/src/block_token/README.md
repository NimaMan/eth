# block_token

Block token processing boundary.

The concrete processor can be backed by `eth_token::tracking::BlockTokenProcessor` and its token cache. This module only defines the live-feed output contract: which token snapshots were updated or removed after a processed confirmed block.

`LiveBlockTokenProcessor` should be a later runtime wrapper, not this low-level per-block contract.
