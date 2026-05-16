use super::{story_section, RiskAtlasStorySection};

pub const DISTRIBUTION_SECTION: &str = "scam_mechanisms";

pub fn section() -> RiskAtlasStorySection {
    story_section(
        "scam_mechanisms",
        "Scam Mechanisms",
        "The mechanism mix behind scam labels, separated from strategy outcomes.",
        Some(DISTRIBUTION_SECTION),
        None,
    )
}
