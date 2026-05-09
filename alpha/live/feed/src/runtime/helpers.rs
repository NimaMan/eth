use super::progress::{LiveTokenProgress, LiveTokenStatus};

pub(super) fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn status_label(status: &LiveTokenStatus) -> &'static str {
    match status {
        LiveTokenStatus::Idle => "idle",
        LiveTokenStatus::Warming => "warming",
        LiveTokenStatus::Live => "live",
        LiveTokenStatus::Stopping => "stopping",
        LiveTokenStatus::Stopped => "stopped",
        LiveTokenStatus::Failed => "failed",
    }
}

pub(super) fn should_log_block_apply(progress: &LiveTokenProgress, is_live_tail: bool) -> bool {
    is_live_tail
        || progress.blocks_processed == 1
        || progress.blocks_processed % 100 == 0
        || (progress.warmup_total_blocks > 0
            && progress.blocks_processed == progress.warmup_total_blocks)
}

pub(super) fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
