use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use eth_alpha_core::ids::TradeId;

static TRADE_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) fn new_trade_id() -> TradeId {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();
    let sequence = TRADE_COUNTER.fetch_add(1, Ordering::Relaxed);
    TradeId(format!(
        "trd_{}_{}_{}",
        base36(millis),
        base36(std::process::id() as u64),
        base36(sequence)
    ))
}

fn base36(mut value: u64) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(match digit {
            0..=9 => b'0' + digit,
            _ => b'a' + (digit - 10),
        });
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_else(|_| "0".to_string())
}
