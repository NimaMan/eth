# Execution

Execution adapters translate approved engine `OrderIntent`s into
`ExecutionReport`s.

- `simulated/`: historical and live no-capital chain-state EVM simulation.
- `real/`: crate-private live tx executor boundary used only by
  `live_trader/real_execution.rs`.
- `sell_economics.rs`: execution-level sell safety checks that depend on final
  simulated proceeds and gas cost. These are not strategy exit rules.
