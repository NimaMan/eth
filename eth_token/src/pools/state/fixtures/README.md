# Fixtures

Fixtures are synthetic tracked states used by tests and docs. They intentionally
layer observations on top of `BasePool` projections so new tracks can be tested
before scanner, bytecode, trace, or simulation adapters are wired into runtime
ingestion.

Fixtures create synthetic `PoolTrackedState` values for examples, tests, and
consumer integration work. They do not represent canonical chain data.

Current fixtures cover Banana Gun pass-through roles, holder-balance custody
drain, and direct LP liquidity pull.
