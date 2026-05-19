use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, U256,
};
use alloy_sol_types::{sol, SolCall, SolValue};
use eyre::{eyre, Result};

use crate::UnsignedTransaction;

use super::{infer_orientation_from_input, UniswapV4PoolKey};

const COMMAND_V4_SWAP: u8 = 0x10;

const ACTION_SWAP_EXACT_IN_SINGLE: u8 = 0x06;
const ACTION_SETTLE: u8 = 0x0b;
const ACTION_TAKE: u8 = 0x0e;

sol! {
    struct PoolKey {
        address currency0;
        address currency1;
        uint24 fee;
        int24 tickSpacing;
        address hooks;
    }

    struct ExactInputSingleParams {
        PoolKey poolKey;
        bool zeroForOne;
        uint128 amountIn;
        uint128 amountOutMinimum;
        bytes hookData;
        uint256 minHopPriceX36;
    }

    function execute(bytes commands, bytes[] inputs, uint256 deadline);
}

/// How the Universal Router should fund the v4 input currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniversalRouterV4InputPayment {
    /// Use native ETH supplied as transaction value. The pool input currency must be address(0).
    NativeEth,
    /// Pull the ERC20 input currency from the caller through Permit2.
    Permit2User,
}

/// Exact-input single-hop v4 swap through the deployed Uniswap Universal Router.
#[derive(Debug, Clone)]
pub struct UniversalRouterV4ExactInputSingleRequest {
    pub universal_router: Address,
    pub caller: Address,
    pub pool_key: UniswapV4PoolKey,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub min_amount_out: U256,
    pub deadline: U256,
    pub hook_data: Vec<u8>,
    pub input_payment: UniversalRouterV4InputPayment,
}

/// Build a Universal Router `execute(bytes,bytes[],uint256)` transaction for a v4 exact-input
/// single-hop swap.
pub fn build_universal_router_v4_exact_input_single_tx(
    request: &UniversalRouterV4ExactInputSingleRequest,
) -> Result<UnsignedTransaction> {
    let orientation = infer_orientation_from_input(&request.pool_key, request.token_in)?;
    if request.token_out != orientation.output_currency {
        return Err(eyre!(
            "token_out {:x} does not match pool currencies (expected {:x})",
            request.token_out,
            orientation.output_currency
        ));
    }

    let amount_in = u256_to_u128(request.amount_in, "amount_in")?;
    let min_amount_out = u256_to_u128(request.min_amount_out, "min_amount_out")?;
    let mut commands = Vec::new();
    let mut inputs = Vec::new();

    match request.input_payment {
        UniversalRouterV4InputPayment::NativeEth => {
            if orientation.input_currency != Address::ZERO {
                return Err(eyre!(
                    "NativeEth payment requires address(0) input currency, got {:x}",
                    orientation.input_currency
                ));
            }
            commands.push(COMMAND_V4_SWAP);
            inputs.push(encode_v4_swap_input(
                request,
                orientation.zero_for_one,
                amount_in,
                min_amount_out,
            )?);
        }
        UniversalRouterV4InputPayment::Permit2User => {
            if orientation.input_currency == Address::ZERO {
                return Err(eyre!(
                    "Permit2User payment requires an ERC20 input currency"
                ));
            }
            commands.push(COMMAND_V4_SWAP);
            inputs.push(encode_v4_swap_input(
                request,
                orientation.zero_for_one,
                amount_in,
                min_amount_out,
            )?);
        }
    }

    let calldata = executeCall {
        commands: Bytes::from(commands),
        inputs,
        deadline: request.deadline,
    }
    .abi_encode();

    Ok(UnsignedTransaction {
        from: Some(request.caller),
        to: Some(request.universal_router),
        gas: Some(2_500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(match request.input_payment {
            UniversalRouterV4InputPayment::Permit2User => U256::ZERO,
            UniversalRouterV4InputPayment::NativeEth => request.amount_in,
        }),
        data: Some(Bytes::from(calldata)),
        nonce: None,
        ..Default::default()
    })
}

fn encode_v4_swap_input(
    request: &UniversalRouterV4ExactInputSingleRequest,
    zero_for_one: bool,
    amount_in: u128,
    min_amount_out: u128,
) -> Result<Bytes> {
    let pool_key = PoolKey {
        currency0: request.pool_key.currency0,
        currency1: request.pool_key.currency1,
        fee: U24::try_from(request.pool_key.fee)
            .map_err(|_| eyre!("fee {} exceeds uint24", request.pool_key.fee))?,
        tickSpacing: I24::try_from(request.pool_key.tick_spacing).map_err(|_| {
            eyre!(
                "tick_spacing {} exceeds int24",
                request.pool_key.tick_spacing
            )
        })?,
        hooks: request.pool_key.hooks,
    };
    let swap_params = ExactInputSingleParams {
        poolKey: pool_key,
        zeroForOne: zero_for_one,
        amountIn: amount_in,
        amountOutMinimum: min_amount_out,
        hookData: Bytes::copy_from_slice(&request.hook_data),
        minHopPriceX36: U256::ZERO,
    };

    let input_currency = request.token_in;
    let output_currency = request.token_out;
    let mut actions = vec![ACTION_SWAP_EXACT_IN_SINGLE];
    let mut params = vec![Bytes::from(swap_params.abi_encode())];

    actions.push(ACTION_SETTLE);
    params.push(Bytes::from(
        (input_currency, request.amount_in, true).abi_encode(),
    ));

    actions.push(ACTION_TAKE);
    params.push(Bytes::from(
        (output_currency, request.caller, U256::ZERO).abi_encode(),
    ));

    Ok(Bytes::from(
        (Bytes::from(actions), params).abi_encode_params(),
    ))
}

fn u256_to_u128(value: U256, label: &str) -> Result<u128> {
    value
        .try_into()
        .map_err(|_| eyre!("{label} exceeds uint128"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(
        input_payment: UniversalRouterV4InputPayment,
    ) -> UniversalRouterV4ExactInputSingleRequest {
        let weth = Address::with_last_byte(1);
        UniversalRouterV4ExactInputSingleRequest {
            universal_router: Address::with_last_byte(0x66),
            caller: Address::with_last_byte(0xaa),
            pool_key: UniswapV4PoolKey {
                currency0: Address::ZERO,
                currency1: weth,
                fee: 500,
                tick_spacing: 10,
                hooks: Address::ZERO,
            },
            token_in: Address::ZERO,
            token_out: weth,
            amount_in: U256::from(1_000_000_000_000_000_000u128),
            min_amount_out: U256::ZERO,
            deadline: U256::from(u64::MAX),
            hook_data: Vec::new(),
            input_payment,
        }
    }

    #[test]
    fn builds_native_eth_universal_router_call() {
        let tx = build_universal_router_v4_exact_input_single_tx(&sample_request(
            UniversalRouterV4InputPayment::NativeEth,
        ))
        .unwrap();
        let data = tx.data.unwrap();

        assert_eq!(tx.value, Some(U256::from(1_000_000_000_000_000_000u128)));
        assert_eq!(&data[..4], executeCall::SELECTOR);
    }

    #[test]
    fn rejects_permit2_when_pool_input_is_native_eth() {
        let err = build_universal_router_v4_exact_input_single_tx(&sample_request(
            UniversalRouterV4InputPayment::Permit2User,
        ))
        .unwrap_err();

        assert!(err.to_string().contains("requires an ERC20 input currency"));
    }

    #[test]
    fn exact_input_single_uses_deployed_router_hook_data_field_order() {
        let swap_params = ExactInputSingleParams {
            poolKey: PoolKey {
                currency0: Address::with_last_byte(1),
                currency1: Address::with_last_byte(2),
                fee: U24::from(100),
                tickSpacing: I24::try_from(1).unwrap(),
                hooks: Address::ZERO,
            },
            zeroForOne: false,
            amountIn: 1_000,
            amountOutMinimum: 900,
            hookData: Bytes::new(),
            minHopPriceX36: U256::from(20),
        };

        let encoded = swap_params.abi_encode();

        assert_eq!(abi_word(&encoded, 0), U256::from(32));
        assert_eq!(abi_word(&encoded, 9), U256::from(0x140));
        assert_eq!(abi_word(&encoded, 10), U256::from(20));
        assert_eq!(abi_word(&encoded, 11), U256::ZERO);
    }

    fn abi_word(data: &[u8], index: usize) -> U256 {
        let start = index * 32;
        U256::from_be_slice(&data[start..start + 32])
    }
}
