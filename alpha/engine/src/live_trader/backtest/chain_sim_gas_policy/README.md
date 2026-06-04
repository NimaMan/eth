# Chain-Sim Gas Policy

This module wraps the live backtest execution adapter with the gas-policy
shadow evidence used by real live trading.

- `mod.rs` owns the adapter wrapper and gas-candidate selection flow.
- `metadata.rs` extracts tail-entry ordering metadata from decision evidence.
- `policy_context.rs` classifies sell intents and gas-rank candidate allowances.
- `route.rs` builds the synthetic planner input and sell route used only for
  gas ranking.
- `shadow_outcome.rs` converts gas-policy selections into mined evidence and
  contains the focused unit tests for that evidence.

This folder must remain behavior-preserving backtest glue. It must not contact
the ETH tx executor or build a broadcastable transaction.
