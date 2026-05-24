pub mod active_targets;
pub mod launch_surface;
pub mod model_readiness;
pub mod page_story;
pub mod review_queues;
pub mod scam_mechanisms;
pub mod time_to_scam;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasStorySection {
    pub id: String,
    pub title: String,
    pub narrative: String,
    pub primary_distribution: Option<String>,
    pub primary_stat_section: Option<String>,
}

pub use page_story::{build_page_story, RiskAtlasPageStory, RiskAtlasPageStoryStep};

pub fn default_story_sections() -> Vec<RiskAtlasStorySection> {
    vec![
        launch_surface::section(),
        scam_mechanisms::section(),
        time_to_scam::section(),
        active_targets::section(),
        review_queues::section(),
        model_readiness::section(),
    ]
}

pub(crate) fn story_section(
    id: &str,
    title: &str,
    narrative: &str,
    primary_distribution: Option<&str>,
    primary_stat_section: Option<&str>,
) -> RiskAtlasStorySection {
    RiskAtlasStorySection {
        id: id.to_string(),
        title: title.to_string(),
        narrative: narrative.to_string(),
        primary_distribution: primary_distribution.map(str::to_string),
        primary_stat_section: primary_stat_section.map(str::to_string),
    }
}
