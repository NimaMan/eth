# Audit

This folder tracks deployment-specific review evidence for the Uniswap V2
trading vault.

The audit gate is broader than Solidity review. It must also cover tx-prep, ETH
tx executor policy, gas-rank/value-cap behavior, and post-deploy operational
controls.

## Files

| File | Purpose |
| --- | --- |
| `checklist.yaml` | Required review checklist before mainnet deployment. |
| `live-go-live-readiness.md` | Required gates before relaxing the ETH tx executor from dry-run to public mempool for live trading. |
| `findings.jsonl` | Append-only findings and resolutions. One JSON object per line. |
| `contract-test-gas-review-2026-05-19.md` | Current contract test and gas review evidence. |
| `../simulations/` | Deployed-vault current-state simulation suite and reports. |

Resolve all high and medium findings before deployment. Any accepted low/info
finding must name the owner and reason in `findings.jsonl`.
