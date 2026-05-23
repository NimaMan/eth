# Scam Pool Labels

This folder documents the token/pool label ledger for supervised scam-risk
analysis. Durable label rows should be written through Rust-owned storage rather
than checked-in flat files.

Labels are token-centric. Strategy performance, strategy entry, and strategy
exit are not label criteria.

## Primary Ledger Contract

Use one row per pool label event. Required columns:

| Column | Meaning |
| --- | --- |
| `chain` | Chain name, currently `ethereum`. |
| `token` | Token address. |
| `pool` | Pool address or stable pool id. |
| `protocol` | Pool protocol when known. |
| `label` | High-level target label. |
| `mechanism` | More specific scam mechanism. |
| `confidence` | `verified`, `probable`, `needs_chain_truth`, or `needs_mechanism_review`. |
| `pool_created_block` | Creation/liquidity block when known. |
| `trading_enabled_block` | First reliable block where public trading was enabled or observed. |
| `trading_enabled_source` | Evidence source for `trading_enabled_block`. |
| `first_observed_trade_block` | First trade block when known. |
| `first_blocked_block` | First sell-blocked block if known. |
| `first_drain_block` | First reserve-drain/bad-liquidity block if known. |
| `label_block` | Earliest reliable block where the label is true. |
| `blocks_from_pool_created_to_label` | `label_block - pool_created_block` when both are known. |
| `blocks_from_trading_enabled_to_label` | `label_block - trading_enabled_block` when both are known. |
| `minutes_from_trading_enabled_to_label` | Approximate Ethereum time using 12 seconds per block. |
| `evidence_ref` | Investigation path or artifact reference. |
| `notes` | Short interpretation. |

Unknown fields stay empty until chain truth is extracted. Do not fill them from
guesswork.

If a historical row includes strategy entry or exit fields, keep them as optional
provenance only. Do not use them to assign `label`, `mechanism`, or timing.

## Label Names

Use the production scam-mechanism labels when available:

- `pair_balance_backdoor_drain`
- `direct_lp_liquidity_removal`
- `reserve_dump_drain`
- `privileged_seller_reserve_drain`
- `sell_blocked_honeypot`
- `chunkable_sell_limit`
- `hidden_mint_supply_expansion`
- `critical_lp_control`
- `unknown_reserve_drain`

If the case is confirmed bad but the mechanism is not finalized, use
`unknown_reserve_drain` and add a note. Legacy rows may still contain
`unclassified_reserve_drain`; normalize those before exporting a model dataset.

## Confidence Names

| Confidence | Meaning |
| --- | --- |
| `verified` | Chain evidence identifies both the bad outcome and the mechanism. |
| `probable` | Bad outcome is clear, but some supporting evidence is incomplete. |
| `needs_chain_truth` | Candidate likely matters, but creation/trade/drain evidence still needs extraction. |
| `needs_mechanism_review` | Bad outcome is verified, but the mechanism cluster is not settled. |
