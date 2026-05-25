# Decision Policy Matrix

This matrix is the operator-facing source of truth for how live strategy
decisions map to gas-rank policy ladders. The implementation reads the ladders
from the root `config.env`; code defaults only exist as library fallbacks and
tests.

## Global Bounds

| Setting | Config key | Current value |
| --- | --- | --- |
| Gas-rank source | `ALPHA_LIVE_GAS_RANK_REQUIRED_SOURCE` | `eth_chain_server_gas_rank` |
| Lookback blocks | `ALPHA_LIVE_GAS_RANK_LOOKBACK_BLOCKS` | `100` |
| Tie breaker | `ALPHA_GAS_RANK_PRIORITY_TIE_BREAKER_GWEI` | `0.1456 gwei` |
| Max priority cap | `ALPHA_LIVE_GAS_MAX_PRIORITY_FEE_GWEI` | `3.5 gwei` |
| Entry estimated fee cap | `ALPHA_LIVE_ENTRY_MAX_ESTIMATED_GAS_FEE_ETH` | `0.0012 ETH` |
| Exit estimated fee cap | `ALPHA_LIVE_EXIT_MAX_ESTIMATED_GAS_FEE_ETH` | `0.002 ETH` |
| V2 vault buy gas limit | `ALPHA_LIVE_UNISWAP_V2_VAULT_BUY_GAS_LIMIT` | `300000` |
| V2 vault sell gas limit | `ALPHA_LIVE_UNISWAP_V2_VAULT_SELL_GAS_LIMIT` | `300000` |
| Mempool-race priority buffer | `ALPHA_LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI` / `ALPHA_LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MAX_GWEI` | `0.1` to `0.2 gwei` |
| Simulated gas buffer | `ALPHA_LIVE_GAS_SIMULATED_GAS_BUFFER_BPS` | `2500 bps` |

## Decision Matrix

| Decision bucket | Strategy reason / source | Execution action | Config key | Ladder | Route | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Entry buy | `entry.buy_eligible_pool_once` | `entry_buy` | `ALPHA_LIVE_ENTRY_BUY_GAS_PROFILES` | `p85 -> p75 -> p50 -> normal` | V2 vault buy | Used before we hold inventory. |
| Normal exit | `exit.max_hold_active_blocks`, restored max-hold exits, take-profit/stop-loss style strategy exits | `normal_exit` | `ALPHA_LIVE_NORMAL_EXIT_GAS_PROFILES` | `p85 -> p75 -> p50 -> normal` | V2 vault sell | Not a mempool race, but starts at `p85` so routine exits do not sit too far back. |
| Mempool LP/removal race | Mempool source with pending tx hash, including `exit.lp_approval` from `mempool_signal` and `exit.mempool_liquidity_removal_signal` | `mempool_race_exit` | `ALPHA_LIVE_MEMPOOL_RACE_EXIT_GAS_PROFILES` | `mempool_race` | Priority V2 vault sell | Reads the triggering mempool tx fee and bids above its effective priority fee with a deterministic per-tx buffer in the configured range. |
| LP approval exit | Mined LP approval or buy-confirm-block LP approval, including `exit.lp_approval_mined_race` and `exit.lp_approval_buy_confirm_block` | `lp_approval_exit` | `ALPHA_LIVE_LP_APPROVAL_EXIT_GAS_PROFILES` | `p90 -> p75 -> p50 -> normal` | Priority V2 vault sell | One shared ladder for confirmed-chain/same-block LP approval exits. |
| Mined liquidity-removal exit | Confirmed `exit.liquidity_removal` from `pool_update` | `mined_liquidity_removal_exit` | `ALPHA_LIVE_LP_APPROVAL_EXIT_GAS_PROFILES` | `p90 -> p75 -> p50 -> normal` | Priority V2 vault sell | Labeled separately from LP approval so mined removals are not mistaken for pending mempool races or LP approvals. |

## Profile Labels

Accepted gas-rank profiles are:

`mempool_race`, `normal`, `p50`, `p55`, `p60`, `p65`, `p70`, `p75`, `p77`, `p85`,
`p88`, `p90`, `p92`, `p94`, `p95`, `p96`, `p97`, `p99`.

Percentile profiles come from the chain-server gas-rank read model. The
`normal` profile is the low-cost rank-target fallback. The `mempool_race`
profile is not a percentile: it is derived from the pending dependency tx's
effective priority fee plus the configured randomized buffer. The selected
profile is still bounded by max-priority and max-total-fee caps; when the first
profile in a ladder is too expensive, the planner tries the next profile.

## Persistence Checks

Persisted execution report evidence should show:

| Evidence field | Expected meaning |
| --- | --- |
| `gas_policy_action` | One of `entry_buy`, `tail_entry_buy`, `normal_exit`, `mempool_race_exit`, `lp_approval_exit`, `mined_liquidity_removal_exit`. |
| `gas_policy_signal` | Original strategy signal/reason, preserved even when multiple reasons share a policy bucket. |
| `gas_policy_profiles` | The configured ladder considered for the decision. |
| `gas_policy_profile` | The actual selected profile after value caps. |
| `gas_rank_source` | Should match `ALPHA_LIVE_GAS_RANK_REQUIRED_SOURCE`. |
| `gas_rank_source = eth_public_mempool_pending_tx_fee` | Expected for `mempool_race` selections; metadata should include dependency tx hash, dependency effective priority fee, selected priority fee, and buffer. |

Example validation query:

```sql
SELECT
  er.trade_id,
  er.order_side,
  er.payload->'mined_evidence'->>'gas_policy_action' AS action,
  er.payload->'mined_evidence'->>'gas_policy_signal' AS signal,
  er.payload->'mined_evidence'->>'gas_policy_profile' AS selected_profile,
  er.payload->'mined_evidence'->'gas_policy_profiles' AS profiles
FROM alpha_trading.execution_reports er
WHERE er.run_id = '<run_id>'
  AND er.status = 'confirmed'
ORDER BY er.created_at DESC
LIMIT 50;
```
