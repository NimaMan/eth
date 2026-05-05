# position

Portfolio position state machine.

## Owns

- `Position`
- `PositionKey`
- `PositionState`
- `PositionSnapshot`
- transition validation

## Does Not Own

- Canonical token state.
- Strategy implementation.
- Executor implementation.
- Database persistence details.

## Python Lesson

Python `TokenPosition` mixed static token facts, market snapshots, strategy state, and position lifecycle. Rust position state should represent our exposure only. Market facts are references/snapshots from `market`.
