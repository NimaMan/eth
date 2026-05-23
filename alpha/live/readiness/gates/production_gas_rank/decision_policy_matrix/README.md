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
| Simulated gas buffer | `ALPHA_LIVE_GAS_SIMULATED_GAS_BUFFER_BPS` | `2500 bps` |

## Decision Matrix

| Decision bucket | Strategy reason / source | Execution action | Config key | Ladder | Route | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Entry buy | `entry.buy_eligible_pool_once` | `entry_buy` | `ALPHA_LIVE_ENTRY_BUY_GAS_PROFILES` | `p85 -> p75 -> p50 -> normal` | V2 vault buy | Used before we hold inventory. |
| Normal exit | `exit.max_hold_active_blocks`, restored max-hold exits, take-profit/stop-loss style strategy exits | `normal_exit` | `ALPHA_LIVE_NORMAL_EXIT_GAS_PROFILES` | `p85 -> p75 -> p50 -> normal` | V2 vault sell | Not a mempool race, but starts at `p85` so routine exits do not sit too far back. |
| Mempool LP/removal race | Mempool source or reason containing `mempool`, including `exit.mempool_liquidity_removal_signal` | `mempool_race_exit` | `ALPHA_LIVE_MEMPOOL_RACE_EXIT_GAS_PROFILES` | `p95 -> p90 -> p75 -> p50 -> normal` | Priority V2 vault sell | Highest urgency public-mempool exit. |
| LP approval exit | Mined LP approval or buy-confirm-block LP approval, including Alpha11 `exit.lp_approval`, `exit.lp_approval_mined_race`, and `exit.lp_approval_buy_confirm_block` | `lp_approval_exit` | `ALPHA_LIVE_LP_APPROVAL_EXIT_GAS_PROFILES` | `p90 -> p75 -> p50 -> normal` | Priority V2 vault sell | One shared ladder for all mined/same-block LP approval exits. |

## Profile Labels

Accepted gas-rank profiles are:

`normal`, `p50`, `p55`, `p60`, `p65`, `p70`, `p75`, `p77`, `p85`,
`p88`, `p90`, `p92`, `p94`, `p95`, `p96`, `p97`, `p99`.

Percentile profiles come from the chain-server gas-rank read model. The
`normal` profile is the low-cost rank-target fallback. The selected profile is
still bounded by max-priority and max-total-fee caps; when the first profile in
a ladder is too expensive, the planner tries the next profile.

## Persistence Checks

Persisted execution report evidence should show:

| Evidence field | Expected meaning |
| --- | --- |
| `gas_policy_action` | One of `entry_buy`, `normal_exit`, `mempool_race_exit`, `lp_approval_exit`. |
| `gas_policy_signal` | Original strategy signal/reason, preserved even when multiple reasons share a policy bucket. |
| `gas_policy_profiles` | The configured ladder considered for the decision. |
| `gas_policy_profile` | The actual selected profile after value caps. |
| `gas_rank_source` | Should match `ALPHA_LIVE_GAS_RANK_REQUIRED_SOURCE`. |

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
