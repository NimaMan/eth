# Tracking Scripts

Scripts in this folder audit live or range token tracking behavior against
chain facts.

## `audit_live_token_tracking.py`

Compares live token tracker pool discovery against pool-creation logs from
known V2, V3, and V4 factories/managers. Use it when checking whether
`eth_token` tracking missed a pool link that exists on chain.
