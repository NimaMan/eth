mod balance_deltas;
mod buyer_setup;
mod entry;
mod failure;
mod fees;
mod results;
mod uniswap_v4;
mod validation;

pub(super) const WETH_DECIMALS: u8 = 18;

pub use entry::check_can_buy_sell_pool;
