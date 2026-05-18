use eyre::{eyre, Result};
use reth_primitives_traits::SealedHeader;

use crate::block_processor::sealed_header_from_processed_block_header;
use crate::trade_simulation::types::PoolBuySellParameters;

pub(in crate::trade_simulation::pool_buy_sell_simulator) fn block_header_hint(
    config: &PoolBuySellParameters,
    block_number: u64,
) -> Result<Option<SealedHeader>> {
    let Some(header) = &config.block_header else {
        return Ok(None);
    };
    if header.number != block_number {
        return Err(eyre!(
            "block header hint mismatch: header={}, requested={}",
            header.number,
            block_number
        ));
    }
    Ok(Some(sealed_header_from_processed_block_header(header)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::B256;
    use reth_chain_query::provider::BlockHeader;

    fn processed_block_header(number: u64) -> BlockHeader {
        BlockHeader {
            number,
            hash: B256::from([1_u8; 32]),
            parent_hash: B256::from([2_u8; 32]),
            timestamp: 1_777_000_000,
            gas_limit: 60_000_000,
            gas_used: 21_000_000,
            base_fee_per_gas: Some(123_456_789),
            withdrawals_root: Some(B256::from([3_u8; 32])),
            blob_gas_used: Some(393_216),
            excess_blob_gas: Some(1_179_648),
            parent_beacon_block_root: Some(B256::from([4_u8; 32])),
            requests_hash: Some(B256::from([5_u8; 32])),
            block_access_list_hash: Some(B256::from([6_u8; 32])),
            slot_number: Some(42),
        }
    }

    #[test]
    fn block_header_hint_uses_processed_block_header() {
        let config =
            PoolBuySellParameters::default().with_block_header(processed_block_header(100));

        let header = block_header_hint(&config, 100)
            .expect("header hint should parse")
            .expect("header should be present");

        assert_eq!(header.number, 100);
        assert_eq!(header.hash(), B256::from([1_u8; 32]));
        assert_eq!(header.parent_hash, B256::from([2_u8; 32]));
        assert_eq!(header.timestamp, 1_777_000_000);
        assert_eq!(header.base_fee_per_gas, Some(123_456_789));
        assert_eq!(header.gas_limit, 60_000_000);
        assert_eq!(header.gas_used, 21_000_000);
    }

    #[test]
    fn block_header_hint_rejects_wrong_block() {
        let config =
            PoolBuySellParameters::default().with_block_header(processed_block_header(100));

        let error = block_header_hint(&config, 101).expect_err("mismatch should fail");

        assert!(error.to_string().contains("block header hint mismatch"));
    }
}
