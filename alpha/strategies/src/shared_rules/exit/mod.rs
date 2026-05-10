//! Strategy-agnostic exit rules.
//!
//! Each rule checks:
//! 1. Does the risk event match the rule's trigger kind?
//! 2. Is there an open, non-terminal position for the affected pool?
//!
//! If both are true, the rule returns `RuleDecision::Exit`. Otherwise it
//! returns `Hold` with a descriptive reason.

pub mod liquidity_removal;
pub mod lp_approval;
pub mod scam;
pub mod tax;
