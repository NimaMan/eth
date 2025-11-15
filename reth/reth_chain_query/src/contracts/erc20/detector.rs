use super::deployment_replay::prepare_state_for_metadata;
use super::metadata::TokenMetadata;
use crate::contracts::common::{build_two_address_payload, call_uint256_view, contains_signature};
use crate::utils::function_signatures::erc20;
use crate::RethQueryProvider;
use alloy_primitives::{Address, Bytes};
use eyre::Result;
use hex_literal::hex;
use reth_primitives::SealedHeader;
use tx_simulator::contract_method_simulator::encode_contract_read_call_with_address_arg;

const TRANSFER_TOPIC: [u8; 32] =
    hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
const APPROVAL_TOPIC: [u8; 32] =
    hex!("8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925");
const ALLOWANCE_SELECTOR: [u8; 4] = hex!("dd62ed3e");

impl RethQueryProvider {
    pub async fn get_token_metadata(
        &self,
        token: Address,
        block_number: Option<u64>,
        block_header: Option<SealedHeader>,
    ) -> Result<Option<TokenMetadata>> {
        self.fetch_metadata_if_erc20(token, block_number, block_header)
            .await
    }

    pub async fn is_erc20_contract(
        &self,
        address: Address,
        block_number: Option<u64>,
        block_header: Option<SealedHeader>,
    ) -> Result<bool> {
        Ok(self
            .fetch_metadata_if_erc20(address, block_number, block_header)
            .await?
            .is_some())
    }

    async fn fetch_metadata_if_erc20(
        &self,
        address: Address,
        block_number: Option<u64>,
        block_header: Option<SealedHeader>,
    ) -> Result<Option<TokenMetadata>> {
        let resolved_block = block_header
            .as_ref()
            .map(|header| header.number)
            .or(block_number)
            .unwrap_or(self.get_latest_block()?);

        prepare_state_for_metadata(self, address, block_number, block_header.clone())?;

        if block_header.is_none() {
            self.simulator().assert_block_available(resolved_block)?;
        }

        let bytecode = self
            .get_contract_bytecode_at_block(address, Some(resolved_block))
            .await?;
        if bytecode.is_empty() {
            return Ok(None);
        }

        if !contains_signature(&bytecode, &TRANSFER_TOPIC)
            || !contains_signature(&bytecode, &APPROVAL_TOPIC)
        {
            return Ok(None);
        }

        let total_supply = match call_uint256_view(
            self,
            address,
            Bytes::copy_from_slice(&erc20::TOTAL_SUPPLY),
            resolved_block,
            block_header.clone(),
        )
        .await?
        {
            Some(value) => value,
            None => return Ok(None),
        };

        let balance_payload =
            encode_contract_read_call_with_address_arg(erc20::BALANCE_OF, Address::ZERO);
        if call_uint256_view(
            self,
            address,
            balance_payload,
            resolved_block,
            block_header.clone(),
        )
        .await?
        .is_none()
        {
            return Ok(None);
        }

        let allowance_payload =
            build_two_address_payload(&ALLOWANCE_SELECTOR, Address::ZERO, Address::ZERO);
        if call_uint256_view(
            self,
            address,
            allowance_payload,
            resolved_block,
            block_header.clone(),
        )
        .await?
        .is_none()
        {
            return Ok(None);
        }

        let header_clone_a = block_header.clone();
        let header_clone_b = block_header.clone();
        let header_clone_c = block_header.clone();
        let (name, symbol, decimals) = tokio::try_join!(
            self.get_token_name(address, Some(resolved_block), header_clone_a),
            self.get_token_symbol(address, Some(resolved_block), header_clone_b),
            self.get_token_decimals(address, Some(resolved_block), header_clone_c),
        )?;

        Ok(Some(TokenMetadata {
            address,
            name,
            symbol,
            decimals,
            total_supply,
        }))
    }
}
