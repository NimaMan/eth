use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use eyre::Result;

use crate::dex::BALANCER_VAULT;
use crate::provider::RethQueryProvider;

/// Function selectors
const GET_POOL_TOKENS_SEL: [u8; 4] = [0xf9, 0x4d, 0x46, 0x68]; // getPoolTokens(bytes32)

impl RethQueryProvider {
    /// Call Balancer Vault.getPool(bytes32) -> (address pool, uint8 specialization)
    pub async fn balancer_v2_get_pool_info(
        &self,
        pool_id: B256,
        block: Option<u64>,
    ) -> Result<(Address, u8)> {
        let selector = &keccak256("getPool(bytes32)".as_bytes())[0..4];
        let mut data = Vec::with_capacity(4 + 32);
        data.extend_from_slice(selector);
        data.extend_from_slice(pool_id.as_slice());
        let res = self
            .simulator()
            .simulate_view_function(BALANCER_VAULT, Bytes::from(data), block, None)
            .await?;
        if !res.success || res.output.len() < 64 {
            return Err(eyre::eyre!("getPool(bytes32) call failed"));
        }
        // address is right-aligned in 32 bytes
        let pool = Address::from_slice(&res.output[12..32]);
        let spec = res.output[63];
        Ok((pool, spec))
    }

    /// Call Balancer Vault.getPoolTokens(bytes32) -> (address[], uint256[], uint256)
    /// Returns (tokens, balances, last_change_block)
    pub async fn balancer_v2_get_pool_tokens_and_balances(
        &self,
        pool_id: B256,
        block: Option<u64>,
    ) -> Result<(Vec<Address>, Vec<U256>, U256)> {
        let mut data = Vec::with_capacity(4 + 32);
        data.extend_from_slice(&GET_POOL_TOKENS_SEL);
        data.extend_from_slice(pool_id.as_slice());
        let res = self
            .simulator()
            .simulate_view_function(BALANCER_VAULT, Bytes::from(data), block, None)
            .await?;
        if !res.success || res.output.len() < 96 {
            return Err(eyre::eyre!("getPoolTokens(bytes32) call failed"));
        }

        // Head layout: [0] offset tokens, [1] offset balances, [2] lastChangeBlock
        let tokens_offset = be_word_to_usize(&res.output[0..32]);
        let balances_offset = be_word_to_usize(&res.output[32..64]);
        let last_change_block = U256::from_be_slice(&res.output[64..96]);

        // Decode tokens array
        let tokens = decode_address_array(&res.output, tokens_offset)
            .ok_or_else(|| eyre::eyre!("failed decoding tokens[]"))?;

        // Decode balances array
        let balances = decode_u256_array(&res.output, balances_offset)
            .ok_or_else(|| eyre::eyre!("failed decoding balances[]"))?;

        Ok((tokens, balances, last_change_block))
    }
}

fn decode_address_array(data: &[u8], offset: usize) -> Option<Vec<Address>> {
    if data.len() < offset + 32 {
        return None;
    }
    let len = be_word_to_usize(&data[offset..offset + 32]);
    let mut out = Vec::with_capacity(len);
    let mut cur = offset + 32;
    for _ in 0..len {
        if data.len() < cur + 32 {
            return None;
        }
        out.push(Address::from_slice(&data[cur + 12..cur + 32]));
        cur += 32;
    }
    Some(out)
}

fn decode_u256_array(data: &[u8], offset: usize) -> Option<Vec<U256>> {
    if data.len() < offset + 32 {
        return None;
    }
    let len = be_word_to_usize(&data[offset..offset + 32]);
    let mut out = Vec::with_capacity(len);
    let mut cur = offset + 32;
    for _ in 0..len {
        if data.len() < cur + 32 {
            return None;
        }
        out.push(U256::from_be_slice(&data[cur..cur + 32]));
        cur += 32;
    }
    Some(out)
}

fn be_word_to_usize(word: &[u8]) -> usize {
    if word.len() < 32 {
        return 0;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&word[24..32]);
    u64::from_be_bytes(bytes) as usize
}
