# token

Tracked-token and pool snapshots.

This module models the portable view of the live token cache. The full token object can remain process-local in `eth_token`; external consumers should read these curated snapshots instead of depending on mutable in-memory token objects.
