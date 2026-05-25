# Display Read Model

Cases that validate how Risk Atlas and Asena should surface token/pool state
without carrying stale liquidity, misleading price ratios, or missing risk
context.

## Questions

Ask these when the chain fact may be right but the page/read-model may mislead:

| Question | Why It Matters |
| --- | --- |
| Which field or card would mislead an operator? | Focuses the case on display/read-model behavior. |
| Is the displayed value stale, computed from dust reserves, missing a caveat, or missing entirely? | Identifies the display failure type. |
| What chain/source fact should the page use instead? | Ties UI output to durable evidence. |
| Should the value be suppressed, labeled as unsafe, replaced with a safer metric, or shown with provenance? | Defines the display rule. |
| Does the read model need a new persisted field, or can it be derived safely at render time? | Keeps frontend logic thin. |
| Is the issue resolved by code/tests/docs? | Resolved display regressions should be removed from Token Lab. |

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| investigating | [sss_space_services_25041048](../../cases/sss_space_services_25041048/) | Chain/token-builder parity is reproduced; display price/init is suppressed for token-reserve-dust pools, and durable observation/model fields are in place. |
