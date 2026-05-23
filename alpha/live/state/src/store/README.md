# store

Reader and writer traits for live-state backends.

The current live runtime uses in-process state. Tests and backtests can use
`InMemoryLiveStateStore`; live services can implement the same traits over a
local runtime-owned store when they need an explicit boundary.
