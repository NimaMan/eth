use super::{story_section, RiskAtlasStorySection};

pub const DISTRIBUTION_SECTION: &str = "time_to_scam_buckets";
pub const STAT_SECTION: &str = "time_to_scam";

pub fn section() -> RiskAtlasStorySection {
    story_section(
        "time_to_scam",
        "Time To Scam",
        "How long pools survive after trading becomes enabled, using both bucket counts and duration percentiles.",
        Some(DISTRIBUTION_SECTION),
        Some(STAT_SECTION),
    )
}
