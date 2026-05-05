# store

Reader and writer traits for live-state backends.

The production backend is expected to be Redis, but the traits do not require Redis. Tests and backtests can use `InMemoryLiveStateStore`, while live services can implement the same traits over Redis pipelines.
