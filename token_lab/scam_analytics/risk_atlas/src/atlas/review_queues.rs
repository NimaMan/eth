use super::{story_section, RiskAtlasStorySection};

pub const QUEUE_NEGATIVE_AGE: &str = "negative_age";
pub const QUEUE_LIQUID_AT_LABEL: &str = "liquid_at_label";
pub const QUEUE_CAN_SELL_AT_LABEL: &str = "can_sell_at_label";
pub const QUEUE_EXTREME_PRICE_RATIO: &str = "extreme_price_ratio";

pub fn section() -> RiskAtlasStorySection {
    story_section(
        "review_queues",
        "Review Queues",
        "Examples that should be inspected before trusting label quality or model conclusions.",
        None,
        None,
    )
}
