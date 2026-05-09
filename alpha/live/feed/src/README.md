# src

Pipeline and runtime code for the confirmed Ethereum live feed.

This crate remains an orchestration boundary over existing processors. It defines how processed blocks, block-token updates, live-state writes, downstream live-feed events, and the Redis-triggered live token runtime connect, without owning transaction signing or pending-mempool simulation.
