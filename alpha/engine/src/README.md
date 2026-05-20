# Engine Source

The engine crate is organized by runtime boundary.

- `bin/`: executable wrappers only.
- `execution/`: execution adapters for simulation and crate-private real live
  submission.
- `live_trader/`: live polling runner and real/live-backtest entrypoint wiring.
- `runtime/`: `AlphaEngine` event handling and execution flow.
- `decision/`: strategy decision persistence.
- `valuation/`: position valuation and snapshot helpers.
- `store/`: engine-local store implementations.
- `wire.rs`: token-server wire types and parsers.
