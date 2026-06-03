# Tracks

Tracks are typed, per-angle state. Each track answers one question and carries
its own evidence references. Tracks are allowed to coexist in combinations that
legacy lifecycle labels cannot express, such as a pool that is still in a
trading lifecycle while custody risk is latent and sell route simulation fails.

The aggregate type is `PoolTrackedState` in `state/model.rs`.
