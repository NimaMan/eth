pub const EVENT_SOURCE_MARKET: &str = "market";
pub const EVENT_SOURCE_MEMPOOL_SIGNAL: &str = "mempool_signal";
pub const EVENT_SOURCE_HISTORICAL_MEMPOOL_SIGNAL: &str = "historical_mempool_signal";
pub const EVENT_SOURCE_RISK_ATLAS_MINED_CHAIN: &str = "risk_atlas_mined_chain";
pub const EVENT_SOURCE_POSITION_MONITOR: &str = "position_monitor";
pub const EVENT_SOURCE_RISK: &str = "risk";

pub fn normalize_source(source: Option<&str>) -> Option<String> {
    let source = source?.trim();
    if source.is_empty() {
        None
    } else {
        Some(source.to_ascii_lowercase())
    }
}
