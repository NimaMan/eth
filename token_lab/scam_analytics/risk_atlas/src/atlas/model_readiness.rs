use super::{story_section, RiskAtlasStorySection};

pub const STATUS_READY: &str = "ready";
pub const STATUS_NEEDS_REVIEW: &str = "needs_review";
pub const STATUS_BLOCKED: &str = "blocked";

pub fn section() -> RiskAtlasStorySection {
    story_section(
        "model_readiness",
        "Trading Path",
        "What is ready for supervised modeling, what still needs label review, and what can later become trading guardrails.",
        None,
        None,
    )
}
