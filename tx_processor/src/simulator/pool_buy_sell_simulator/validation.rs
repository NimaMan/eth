use alloy_primitives::{Address, U256};
use eyre::{eyre, Result};
use reth_chain_query::dex::encoding::{
    encode_function_call, encode_two_addresses, encode_two_addresses_and_uint256,
};
use tx_simulator::{UnsignedTxChainSimulation, ViewFunctionResult};

use crate::simulator::types::{PoolBuySellParameters, PoolType};

const UNISWAP_V2_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];
const UNISWAP_V3_FACTORY_GET_POOL: [u8; 4] = [0x16, 0x98, 0xee, 0x82];

pub(super) fn validate_pool_registration(
    chain: &mut UnsignedTxChainSimulation,
    config: &PoolBuySellParameters,
    block_number: u64,
) -> Result<()> {
    if let Some(protocol) = config.pool_type.known_v2_protocol() {
        let denom = config.denom_address;
        if denom.is_zero() {
            return Err(eyre!(
                "denom_address missing for {:?} pool",
                config.pool_type
            ));
        }
        let resolved = fetch_uniswap_v2_pair_address_on_chain(
            chain,
            protocol.factory(),
            config.token_address,
            denom,
            block_number,
            config.pool_type,
        )?;

        if resolved.is_zero() {
            return Err(eyre!(
                "No {:?} pool found for token {} with denom {} at block {}",
                config.pool_type,
                config.token_address,
                denom,
                block_number
            ));
        }

        if resolved != config.pool_address {
            return Err(eyre!(
                "{:?} factory reports pool {} but configuration provided {} for token {} / denom {} at block {}",
                config.pool_type,
                resolved,
                config.pool_address,
                config.token_address,
                denom,
                block_number
            ));
        }
        return Ok(());
    }

    if let (Some(protocol), Some(fee_tier)) = (
        config.pool_type.known_v3_protocol(),
        config.pool_type.v3_fee_tier(),
    ) {
        let denom = config.denom_address;
        if denom.is_zero() {
            return Err(eyre!(
                "denom_address missing for {} pool checks",
                protocol.label()
            ));
        }

        let resolved = fetch_uniswap_v3_pool_address_on_chain(
            chain,
            protocol.factory(),
            config.token_address,
            denom,
            fee_tier,
            block_number,
            protocol.label(),
        )?;

        if resolved.is_zero() {
            return Err(eyre!(
                "No {} pool found for token {} with denom {} at fee tier {} and block {}",
                protocol.label(),
                config.token_address,
                denom,
                fee_tier,
                block_number
            ));
        }

        if resolved != config.pool_address {
            return Err(eyre!(
                "{} factory reports pool {} but configuration provided {} for token {} / denom {} at fee tier {} and block {}",
                protocol.label(),
                resolved,
                config.pool_address,
                config.token_address,
                denom,
                fee_tier,
                block_number
            ));
        }

        return Ok(());
    }

    Ok(())
}

fn fetch_uniswap_v2_pair_address_on_chain(
    chain: &mut UnsignedTxChainSimulation,
    factory: Address,
    token_a: Address,
    token_b: Address,
    block_number: u64,
    pool_type: PoolType,
) -> Result<Address> {
    let params = encode_two_addresses(token_a, token_b);
    let call_data = encode_function_call(UNISWAP_V2_FACTORY_GET_PAIR, &params);
    let response = chain
        .simulate_view_call(factory, call_data)
        .map_err(|err| {
            eyre!(
                "{:?} factory getPair check failed at block {}: {}",
                pool_type,
                block_number,
                err
            )
        })?;

    decode_factory_address_response(
        response,
        format!("{:?} factory getPair", pool_type),
        factory,
        token_a,
        token_b,
        Some(block_number),
    )
}

fn fetch_uniswap_v3_pool_address_on_chain(
    chain: &mut UnsignedTxChainSimulation,
    factory: Address,
    token_a: Address,
    token_b: Address,
    fee_tier: u32,
    block_number: u64,
    protocol_label: &str,
) -> Result<Address> {
    let params = encode_two_addresses_and_uint256(token_a, token_b, U256::from(fee_tier));
    let call_data = encode_function_call(UNISWAP_V3_FACTORY_GET_POOL, &params);
    let response = chain
        .simulate_view_call(factory, call_data)
        .map_err(|err| {
            eyre!(
                "{} factory getPool check failed at block {}: {}",
                protocol_label,
                block_number,
                err
            )
        })?;

    decode_factory_address_response(
        response,
        format!("{protocol_label} factory getPool fee={fee_tier}"),
        factory,
        token_a,
        token_b,
        Some(block_number),
    )
}

fn decode_factory_address_response(
    response: ViewFunctionResult,
    label: String,
    factory: Address,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Address> {
    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "{} reverted (factory={}, token_a={}, token_b={}, block={:?}; {})",
            label,
            factory,
            token_a,
            token_b,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "{} returned empty output (factory={}, token_a={}, token_b={}, block={:?})",
            label,
            factory,
            token_a,
            token_b,
            block_number
        ));
    }

    Ok(Address::from_slice(&response.output[12..32]))
}
