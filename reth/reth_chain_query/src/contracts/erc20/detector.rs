use super::deployment_replay::prepare_state_for_metadata;
use super::metadata::TokenMetadata;
use crate::contracts::common::{build_two_address_payload, call_uint256_view, contains_signature};
use crate::utils::function_signatures::erc20;
use crate::RethQueryProvider;
use alloy_primitives::{Address, Bytes, B256};
use eyre::Result;
use hex_literal::hex;
use tx_simulator::{
    contract_method_simulator::{
        decode_string_from_contract_output, encode_contract_read_call_with_address_arg,
    },
    types::ViewFunctionResult,
    UnsignedTxChainSimulation,
};

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
        pending_tx_hashes: Option<Vec<B256>>,
    ) -> Result<Option<TokenMetadata>> {
        let hashes = pending_tx_hashes.unwrap_or_default();
        self.fetch_metadata_if_erc20(token, block_number, &hashes)
            .await
    }

    pub async fn is_erc20_contract(
        &self,
        address: Address,
        block_number: Option<u64>,
        pending_tx_hashes: Option<Vec<B256>>,
    ) -> Result<bool> {
        let hashes = pending_tx_hashes.unwrap_or_default();
        Ok(self
            .fetch_metadata_if_erc20(address, block_number, &hashes)
            .await?
            .is_some())
    }

    async fn fetch_metadata_if_erc20(
        &self,
        address: Address,
        block_number: Option<u64>,
        pending_tx_hashes: &[B256],
    ) -> Result<Option<TokenMetadata>> {
        let resolved_block = block_number.unwrap_or(self.get_latest_block()?);

        let mut pending_chain =
            prepare_state_for_metadata(self, block_number, pending_tx_hashes).await?;

        // Rely on the simulator's ChainDataLoader to hydrate the block context. If neither MDBX
        // nor the live cache can supply the requested block, the subsequent view calls will fail
        // with a descriptive error.

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
            pending_chain.as_mut(),
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
            pending_chain.as_mut(),
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
            pending_chain.as_mut(),
        )
        .await?
        .is_none()
        {
            return Ok(None);
        }

        let (name, symbol, decimals) = if let Some(chain) = pending_chain.as_mut() {
            let name_res = self
                .execute_view_call_with_pending(
                    address,
                    Bytes::copy_from_slice(&erc20::NAME),
                    resolved_block,
                    Some(chain),
                )
                .await?;
            let symbol_res = self
                .execute_view_call_with_pending(
                    address,
                    Bytes::copy_from_slice(&erc20::SYMBOL),
                    resolved_block,
                    Some(chain),
                )
                .await?;
            let decimals_value = call_uint256_view(
                self,
                address,
                Bytes::copy_from_slice(&erc20::DECIMALS),
                resolved_block,
                Some(chain),
            )
            .await?;

            let decimals = match decimals_value {
                Some(value) => value.to::<u8>(),
                None => return Ok(None),
            };

            (
                decode_string_from_contract_output(&name_res.output),
                decode_string_from_contract_output(&symbol_res.output),
                decimals,
            )
        } else {
            tokio::try_join!(
                self.get_token_name(address, Some(resolved_block)),
                self.get_token_symbol(address, Some(resolved_block)),
                self.get_token_decimals(address, Some(resolved_block)),
            )?
        };

        Ok(Some(TokenMetadata {
            address,
            name,
            symbol,
            decimals,
            total_supply,
        }))
    }

    async fn execute_view_call_with_pending(
        &self,
        contract: Address,
        data: Bytes,
        block_number: u64,
        chain: Option<&mut UnsignedTxChainSimulation>,
    ) -> Result<ViewFunctionResult> {
        if let Some(chain) = chain {
            chain.simulate_view_call(contract, data)
        } else {
            self.simulate_contract_view_call(contract, data, Some(block_number))
                .await
        }
    }
}
