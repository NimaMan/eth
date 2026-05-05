eth_price_leverage (Python)

Lightweight Python package for price-leverage analysis helpers that operate on data produced by the Rust/PyReth stack.

The old Python RL env/action/agent wrappers around retired `PyStablecoinEnv` and `PyStablecoinAction` bindings have been removed. Reintroduce those workflows only through first-class PyReth bindings.

Structure
- policies/best_venue: Venue scoring helpers for price batches
- examples/analytics: Plotting and analysis scripts
- config: Typed configs for env/training

Quick start
Import the scoring helpers directly:

```python
from eth_price_leverage.policies.best_venue import BestVenueScorer
```

Notes
- Block, tx, simulation, and env execution should be owned by Rust/PyReth bindings rather than Python wrapper layers.
