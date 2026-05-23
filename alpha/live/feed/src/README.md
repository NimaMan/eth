# src

Pipeline and runtime code for the confirmed Ethereum live feed.

This crate remains an orchestration boundary over existing processors. It
defines how processed blocks, block-token updates, live-state writes, and
downstream live-feed events connect. It does not own transaction signing or
pending-mempool simulation.
