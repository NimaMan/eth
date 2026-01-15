# Supervised Learning

This package contains the ETH price forecasting stack built on node-native information. Modules under this namespace coordinate data preparation, model training, evaluation, and scheduled inference tasks.

- `data_prep/` defines the canonical information contract and the readers that hydrate it from local chain data.
- `datasets/` contains dataset assembly logic and schema validations for supervised targets.
- `models/` houses reusable model definitions and hyperparameter utilities.
- `evaluation/` provides scoring, diagnostics, and reporting helpers.
- `orchestration/` wires the pieces together for backfills, periodic retraining, and live refresh loops.
- `targets/` describes prediction targets and their computation semantics.
