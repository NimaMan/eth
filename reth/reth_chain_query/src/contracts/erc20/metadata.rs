use crate::RethQueryProvider;
use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result};
use tx_simulator::contract_method_simulator::{
    decode_string_from_contract_output, decode_uint256_from_contract_output,
    encode_contract_read_call_with_address_arg,
};
use tx_simulator::ViewFunctionResult;

impl RethQueryProvider {
    pub(crate) async fn get_token_balance_internal(
        &self,
        token: Address,
        holder: Address,
        block_number: Option<u64>,
    ) -> Result<U256> {
        let selector = [0x70, 0xa0, 0x82, 0x31];
        let data = encode_contract_read_call_with_address_arg(selector, holder);

        let result = self
            .simulate_contract_view_call(token, data, block_number)
            .await?;

        if result.success {
            Ok(decode_uint256_from_contract_output(&result.output))
        } else {
            Ok(U256::ZERO)
        }
    }

    pub async fn get_token_allowance(
        &self,
        token: Address,
        owner: Address,
        spender: Address,
        block_number: Option<u64>,
    ) -> Result<U256> {
        let mut payload = Vec::with_capacity(4 + 32 + 32);
        payload.extend_from_slice(&[0xdd, 0x62, 0xed, 0x3e]);
        payload.extend_from_slice(&[0u8; 12]);
        payload.extend_from_slice(owner.as_slice());
        payload.extend_from_slice(&[0u8; 12]);
        payload.extend_from_slice(spender.as_slice());

        let result = self
            .simulate_contract_view_call(token, Bytes::from(payload), block_number)
            .await?;

        if result.success {
            Ok(decode_uint256_from_contract_output(&result.output))
        } else {
            Ok(U256::ZERO)
        }
    }

    pub async fn get_token_total_supply(
        &self,
        token: Address,
        block_number: Option<u64>,
    ) -> Result<U256> {
        let selector = [0x18, 0x16, 0x0d, 0xdd];
        let data = Bytes::from(selector.to_vec());

        let result = self
            .simulate_contract_view_call(token, data, block_number)
            .await?;

        if result.success {
            Ok(decode_uint256_from_contract_output(&result.output))
        } else {
            Err(eyre!("Failed to get total supply"))
        }
    }

    pub async fn get_token_decimals(
        &self,
        token: Address,
        block_number: Option<u64>,
    ) -> Result<u8> {
        let selector = [0x31, 0x3c, 0xe5, 0x67];
        let data = Bytes::from(selector.to_vec());

        let result = self
            .simulate_contract_view_call(token, data, block_number)
            .await?;

        if !result.success {
            tracing::warn!(
                target: "reth_chain_query::contract_methods",
                token = %token,
                block = ?block_number,
                "decimals() call reverted"
            );
            return Err(eyre!("Failed to get token decimals for {token:?}"));
        }

        if result.output.len() < 32 {
            tracing::warn!(
                target: "reth_chain_query::contract_methods",
                token = %token,
                block = ?block_number,
                returned_bytes = result.output.len(),
                "decimals() call returned insufficient data"
            );
            return Err(eyre!(
                "Token decimals call for {token:?} returned {} bytes (expected >= 32)",
                result.output.len()
            ));
        }

        Ok(result.output[31])
    }

    pub async fn get_token_symbol(
        &self,
        token: Address,
        block_number: Option<u64>,
    ) -> Result<String> {
        let selector = [0x95, 0xd8, 0x9b, 0x41];
        let data = Bytes::from(selector.to_vec());

        let result = self
            .simulate_contract_view_call(token, data, block_number)
            .await?;

        if !result.success {
            tracing::warn!(
                target: "reth_chain_query::contract_methods",
                token = %token,
                block = ?block_number,
                "symbol() call reverted"
            );
            return Err(eyre!("Failed to get token symbol for {token:?}"));
        }

        Ok(decode_string_from_contract_output(&result.output))
    }

    pub async fn get_token_name(
        &self,
        token: Address,
        block_number: Option<u64>,
    ) -> Result<String> {
        let selector = [0x06, 0xfd, 0xde, 0x03];
        let data = Bytes::from(selector.to_vec());

        let result = self
            .simulate_contract_view_call(token, data, block_number)
            .await?;

        if !result.success {
            tracing::warn!(
                target: "reth_chain_query::contract_methods",
                token = %token,
                block = ?block_number,
                "name() call reverted"
            );
            return Err(eyre!("Failed to get token name for {token:?}"));
        }

        Ok(decode_string_from_contract_output(&result.output))
    }

    pub async fn execute_contract_method(
        &self,
        contract: Address,
        method_name: &str,
        _args: impl AsRef<[u8]>,
        block_number: Option<u64>,
    ) -> Result<U256> {
        let data = match method_name {
            "totalSupply" => Bytes::from(vec![0x18, 0x16, 0x0d, 0xdd]),
            "decimals" => Bytes::from(vec![0x31, 0x3c, 0xe5, 0x67]),
            "balanceOf" => {
                return Err(eyre!("balanceOf requires proper argument encoding"));
            }
            _ => return Err(eyre!("Method {} not yet supported", method_name)),
        };

        let result = self
            .simulate_contract_view_call(contract, data, block_number)
            .await?;

        if result.success {
            Ok(decode_uint256_from_contract_output(&result.output))
        } else {
            Err(eyre!("Contract method {} failed", method_name))
        }
    }

    pub(crate) async fn simulate_contract_view_call(
        &self,
        contract: Address,
        data: Bytes,
        block_number: Option<u64>,
    ) -> Result<ViewFunctionResult> {
        self.simulator()
            .simulate_contract_read_only_call_with_options(contract, data, block_number, None)
            .await
    }
}

#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
}
