# Decision Rationale

Structured decision-rationale metadata for strategy decisions.

This module keeps the old free-form `reason` string compatible while producing
stable fields for debugging and aggregation:

- `code.rs`: canonical reason-code normalization and labels.
- `category.rs`: coarse buckets such as entry, exit, hold, risk-policy, and execution.
- `source.rs`: event-source normalization.
- `decision_reason.rs`: the exported `DecisionReason` model persisted with decisions.

Use this module when a strategy, engine, store, or UI needs to explain why a
decision was made. Execution failures and raw risk messages can keep their own
domain payloads, but any strategy decision should normalize through this layer.
