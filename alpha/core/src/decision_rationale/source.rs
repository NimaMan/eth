pub const EVENT_SOURCE_POOL_UPDATE: &str = "pool_update";
pub const EVENT_SOURCE_MEMPOOL_SIGNAL: &str = "mempool_signal";
pub const EVENT_SOURCE_MANUAL_CLOSE: &str = "manual_close";
pub const EVENT_SOURCE_RISK: &str = "risk";

pub fn normalize_source(source: Option<&str>) -> Option<String> {
    let source = source?.trim();
    if source.is_empty() {
        None
    } else {
        Some(source.to_ascii_lowercase())
    }
}
