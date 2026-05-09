# manager

Compatibility facade for the old `eth_token::manager` module path.

The implementation moved to `eth_token::tracking`. Keep this facade until all
callers have been migrated, then remove it in a later cleanup.

New code should import from `eth_token::tracking`.
