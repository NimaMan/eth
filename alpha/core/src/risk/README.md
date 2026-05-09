# risk

Speculative risk and policy decisions.

## Owns

- `RiskEvent`
- `RiskDecision`
- `RiskPolicy`

## Does Not Own

- Mempool polling or simulation.
- Canonical token/pool state.
- Direct order submission.

## Python Lesson

Scam and mempool findings should be inputs to the engine, not hidden mutations inside token or position objects. Risk can advise: allow, block, reduce, cancel, or force exit.
