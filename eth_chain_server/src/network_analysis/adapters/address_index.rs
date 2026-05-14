use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::Address;
use eth_token::network::flow_context::index_query::AddressParticipationQuery;
use eth_token::network::flow_context::model::BlockRange;
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;

#[derive(Clone)]
pub struct RethIndexAddressParticipationQuery {
    provider: Arc<RethQueryProvider>,
}

impl RethIndexAddressParticipationQuery {
    pub fn new(provider: Arc<RethQueryProvider>) -> Self {
        Self { provider }
    }
}

impl AddressParticipationQuery for RethIndexAddressParticipationQuery {
    fn blocks_for_address(&self, address: &str, range: BlockRange) -> Result<Vec<u64>> {
        let address = Address::from_str(address)
            .map_err(|error| eyre!("invalid indexed address {address}: {error}"))?;
        let mut blocks = self.provider.get_address_participation_blocks(address)?;
        blocks.retain(|block| range.contains(*block));
        blocks.sort_unstable();
        Ok(blocks)
    }
}
