# Data Preparation

`data_prep` organizes how raw node-native sources are transformed into modelling-ready information for supervised learning.

- `information_schema/` enumerates every information slice, including naming, units, cadence, and dependencies.
- `information_extractors/` contains reader implementations that hydrate the schema from the local Reth-backed toolchain.
- `information_warehouse/` manages persistence, historical backfills, and snapshot loading into downstream pipelines.
