# Orchestration

Scheduling and workflow orchestration tying data prep, model training, and inference together. Components may include:
- CLI or daemon entry points for historical backfills and periodic retrains
- dependency graphs ensuring schema updates propagate correctly
- monitoring hooks for job health and data freshness
