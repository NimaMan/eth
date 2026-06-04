//! Core rules — strategy-agnostic and available to every strategy.
//!
//! Entry rules filter pools before strategy-specific logic. Exit rules are the
//! fundamental shared protections every strategy composes: regardless of
//! strategy, a mempool (or mined) liquidity-removal signal, a scam signal, a
//! tax signal, or an LP-approval signal means get out. The engine evaluates
//! each independently so per-rule effect can be quantified; the [`super::config`]
//! flags control which are enabled, but the rules themselves stay shared here.

pub mod entry;
pub mod exit;
pub mod lp_approval;
pub mod lp_approval_warning_exit;
