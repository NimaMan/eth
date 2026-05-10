//! Cross-token operator tracing.
//!
//! Detects addresses that operate across multiple tokens with similar
//! patterns (repeat scammers, launch groups, sniper bots).

pub mod operator;
pub mod overlap;
