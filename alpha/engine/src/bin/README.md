# Binaries

This folder contains executable wrappers only. Shared runtime logic must live in
library modules such as `runtime/` or `execution/`. The live trading binaries
(`eth_alpha_live_trader`, `eth_alpha_live_backtest_trader`) and the `live_trader/`
runtime now live in the `eth_alpha_live_runner` crate (`alpha/live/runner`).
