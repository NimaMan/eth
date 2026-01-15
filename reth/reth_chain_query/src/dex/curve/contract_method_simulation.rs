use alloy_primitives::{keccak256, Address, Bytes, U256};
use eyre::Result;

use crate::provider::RethQueryProvider;

// Function selectors (computed on the fly where convenient)

impl RethQueryProvider {
    /// Read Curve V1 coin address at index using coins/underlying_coins with uint256 or int128
    pub async fn curve_v1_get_coin(
        &self,
        pool: Address,
        index: u8,
        use_underlying: bool,
        block: Option<u64>,
    ) -> Result<Address> {
        // Try underlying_coins first if requested
        if use_underlying {
            if let Some(addr) = self
                .curve_try_get_address(pool, "underlying_coins(uint256)", index, block)
                .await?
            {
                return Ok(addr);
            }
            if let Some(addr) = self
                .curve_try_get_address(pool, "underlying_coins(int128)", index, block)
                .await?
            {
                return Ok(addr);
            }
        }
        // Fallback to base coins()
        if let Some(addr) = self
            .curve_try_get_address(pool, "coins(uint256)", index, block)
            .await?
        {
            return Ok(addr);
        }
        if let Some(addr) = self
            .curve_try_get_address(pool, "coins(int128)", index, block)
            .await?
        {
            return Ok(addr);
        }
        Ok(Address::ZERO)
    }

    /// Read Curve V1 balance for coin index using balances(uint256) or balances(int128)
    pub async fn curve_v1_get_balance(
        &self,
        pool: Address,
        index: u8,
        block: Option<u64>,
    ) -> Result<U256> {
        if let Some(val) = self
            .curve_try_get_u256(pool, "balances(uint256)", index, block)
            .await?
        {
            return Ok(val);
        }
        if let Some(val) = self
            .curve_try_get_u256(pool, "balances(int128)", index, block)
            .await?
        {
            return Ok(val);
        }
        Ok(U256::ZERO)
    }

    /// Find index for a token by scanning up to max_coins
    pub async fn curve_v1_find_coin_index(
        &self,
        pool: Address,
        token: Address,
        use_underlying: bool,
        max_coins: u8,
        block: Option<u64>,
    ) -> Result<Option<u8>> {
        for i in 0..max_coins {
            let coin = self
                .curve_v1_get_coin(pool, i, use_underlying, block)
                .await?;
            if coin == Address::ZERO {
                continue;
            }
            if coin == token {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    async fn curve_try_get_address(
        &self,
        pool: Address,
        sig: &str,
        index: u8,
        block: Option<u64>,
    ) -> Result<Option<Address>> {
        let selector = &keccak256(sig.as_bytes())[0..4];
        let mut data = Vec::with_capacity(4 + 32);
        data.extend_from_slice(selector);
        let mut idx = [0u8; 32];
        idx[31] = index;
        data.extend_from_slice(&idx);
        let res = self
            .simulator()
            .simulate_view_function(pool, Bytes::from(data), block)
            .await?;
        if !res.success || res.output.len() < 32 {
            return Ok(None);
        }
        Ok(Some(Address::from_slice(&res.output[12..32])))
    }

    async fn curve_try_get_u256(
        &self,
        pool: Address,
        sig: &str,
        index: u8,
        block: Option<u64>,
    ) -> Result<Option<U256>> {
        let selector = &keccak256(sig.as_bytes())[0..4];
        let mut data = Vec::with_capacity(4 + 32);
        data.extend_from_slice(selector);
        let mut idx = [0u8; 32];
        idx[31] = index;
        data.extend_from_slice(&idx);
        let res = self
            .simulator()
            .simulate_view_function(pool, Bytes::from(data), block)
            .await?;
        if !res.success || res.output.len() < 32 {
            return Ok(None);
        }
        Ok(Some(U256::from_be_slice(&res.output[0..32])))
    }
}
