# Display Read Model

Cases that validate how Risk Atlas and Asena should surface token/pool state
without carrying stale liquidity, misleading price ratios, or missing risk
context.

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| investigating | [sss_space_services_25041048](../../cases/sss_space_services_25041048/) | Chain/token-builder parity is reproduced; display price/init is suppressed for token-reserve-dust pools, and durable observation/model fields are in place. |
| fixed | [mothman_live_retention_liquidity_removal_25063339](../../cases/mothman_live_retention_liquidity_removal_25063339/) | Live retention now preserves evidence-bearing depleted liquidity-removal pools. |
