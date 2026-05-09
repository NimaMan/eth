//! Generic helpers for token-state modules.

pub mod bounded_history;
pub mod numeric;

pub use bounded_history::{append_to_index_map_history, append_with_history_limit};
pub use numeric::{
    parse_raw_f64, parse_raw_f64_or, parse_raw_i128, parse_raw_i128_or, parse_raw_u128,
    parse_raw_u128_or, scale_raw_units,
};
