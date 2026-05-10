//! Baseline strategies.
//!
//! These are simple, non-selective strategies used to establish a performance
//! floor and validate backtest infrastructure. They buy every eligible pool
//! and serve as the reference against which selective strategies are measured.

pub mod snipe_all;
