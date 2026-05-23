use super::{story_section, RiskAtlasStorySection};

pub const TARGET_FAMILY: &str = "active_observation_horizons";
pub const HORIZONS: [u16; 5] = [1, 2, 3, 5, 10];

pub fn section() -> RiskAtlasStorySection {
    story_section(
        "active_targets",
        "Near-Future Targets",
        "The first model target is scam risk within the next 1, 2, 3, 5, or 10 active token/pool observations.",
        None,
        Some(TARGET_FAMILY),
    )
}
