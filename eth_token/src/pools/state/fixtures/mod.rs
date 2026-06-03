//! Synthetic tracked-state fixtures for examples and tests.

pub mod banana_gun;
pub mod behavior_risks;
pub mod custody_drain;
pub mod lp_pull;

pub use banana_gun::banana_gun_pass_through_state;
pub use behavior_risks::{
    policy_restricted_routeable_state, reused_confiscation_pattern_state,
    tax_policy_without_custody_state,
};
pub use custody_drain::custody_drain_state;
pub use lp_pull::lp_pull_state;
