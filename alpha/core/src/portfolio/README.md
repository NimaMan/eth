# portfolio

Portfolio state, limits, and aggregate metrics.

## Owns

- `PortfolioState`
- `PortfolioLimits`
- `PortfolioMetrics`

## Does Not Own

- Strategy-specific decision rules.
- Exchange routing.
- Database implementation.

## Python Lesson

Python metrics were computed from JSON-heavy position records. Rust should keep metrics derivable from typed `PositionSnapshot`s and explicit portfolio state.
