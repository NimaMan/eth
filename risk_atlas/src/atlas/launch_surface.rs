use super::{story_section, RiskAtlasStorySection};

pub const DISTRIBUTION_SECTION: &str = "pool_eligibility";

pub fn section() -> RiskAtlasStorySection {
    story_section(
        "launch_surface",
        "Launch Surface",
        "How many token pools entered the range, how many became labelable scams, and how many remain eligible controls.",
        Some(DISTRIBUTION_SECTION),
        None,
    )
}
