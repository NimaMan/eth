# Engine Source

The engine crate is organized by runtime boundary.

- `bin/`: executable wrappers only (`eth_alpha_tx_executor_calibrate`).
- `execution/`: execution adapters for simulation plus the `real` live-submission
  adapters (`execution::real`), consumed by the `eth_alpha_live_runner` crate.
- `runtime/`: `AlphaEngine` event handling and execution flow.
- `decision/`: strategy decision persistence.
- `valuation/`: position valuation and snapshot helpers.
- `store/`: engine-local store implementations.
- `wire.rs`: token-server wire types and parsers.
