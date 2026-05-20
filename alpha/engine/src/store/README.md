# Store

Store contains engine-local `TradingStore` implementations.

`MemoryTradingStore` is for tests and lightweight runtime scaffolding. Durable
production state belongs to the Postgres store crate.
