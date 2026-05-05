# src

Pipeline contracts for confirmed Ethereum market data.

This crate should remain an orchestration boundary over existing processors. It defines how processed blocks, block-token updates, live-state writes, and downstream events connect, without owning Redis or transaction signing.
