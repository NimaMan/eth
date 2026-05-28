# Token Retention Policies

Policy names describe behavior, not the caller that happens to use the policy.

| Name | Behavior | Primary use |
| --- | --- | --- |
| `keep_all` | Unbounded token index. No terminal, idle, or LRU token drops. | Small ranges and correctness/debug runs. |
| `lru_cache` | Bounded token index using least-recently-updated token eviction when the index exceeds its configured size. | Live/UI memory control when old tokens can be evicted without a terminal decision. |
| `terminal_scam_immediate` | Unbounded token index with terminal scam tokens dropped after the current block's observations are collected. | Run-only scam analytics where post-terminal behavior is not needed. |
| `terminal_or_idle_50k` | Unbounded token index. Terminal scams and liquidity-removal pools are retained for 50,000 blocks after terminal evidence. Other tokens are dropped after 50,000 blocks without meaningful activity. | Historical PnL runs that need a final aggregate snapshot before dropping token state. |

`bounded_index` and `ephemeral_terminal_scam` are old serialized names accepted
as compatibility aliases for `lru_cache` and `terminal_scam_immediate`. New code
and docs should use the names above.

For PnL history, retention is a cache lifecycle decision. The runner must export
aggregate PnL before applying a drop so the database stores the latest state
without persisting per-movement rows.
