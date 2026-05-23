# Token Retention Policies

Policy types are named by behavior, not by the caller that uses them.

- `keep_all`: unbounded token index, no terminal-state pruning.
- `bounded_index`: capped token index using normal LRU eviction.
- `ephemeral_terminal_scam`: unbounded token index plus immediate terminal scam pruning after the current block's observations are collected.

`ephemeral_terminal_scam` is intended for run-only analytics where the result
only needs aggregate observations and should not retain full scam token state
after the scam has been observed.
