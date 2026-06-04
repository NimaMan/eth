# Execution

Execution adapters translate approved engine `OrderIntent`s into
`ExecutionReport`s.

- `simulated/`: historical and live no-capital chain-state EVM simulation.
- `real/`: live tx executor boundary (`#[doc(hidden)] pub`), used only by the
  `eth_alpha_live_runner` crate's `live_trader/real_execution` wiring.
- `sell_economics.rs`: execution-level sell safety checks that depend on final
  simulated proceeds and gas cost. These are not strategy exit rules.
