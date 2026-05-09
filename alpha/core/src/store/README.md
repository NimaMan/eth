# store

Persistence contract for the trading domain.

## Owns

- `TradingStore` trait.

## Does Not Own

- PostgreSQL connection pools.
- Redis implementations.
- SQL schema migrations.
- JSON compatibility adapters.

## Python Lesson

Python writers were tightly coupled to position objects and live/backtest code paths. Rust core should define a small store contract; concrete stores belong in engine/infrastructure crates.
