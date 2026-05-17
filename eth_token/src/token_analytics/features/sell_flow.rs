#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ObservedSellTransferFlow {
    pub observed_sell_tx_count: u32,
    pub seller_token_out: f64,
    pub seller_token_to_pool: f64,
    pub seller_token_to_token_contract: f64,
    pub seller_token_to_other: f64,
    pub token_contract_to_pool: f64,
    pub pool_token_reserve: Option<f64>,
}
