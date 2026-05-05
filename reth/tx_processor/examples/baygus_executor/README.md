Baygus Executor Examples

This folder contains compile-checked examples for building BaygusExecutor transactions from the maintained Rust API.

Use `tx_simulator::tx_builders::baygus_executor::BaygusExecutionPlan` for command composition. Do not hand-encode `execute(bytes,bytes[])` calldata in examples; ABI details belong in `tx_simulator`.
