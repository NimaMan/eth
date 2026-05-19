use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, U256,
};
use alloy_sol_types::{sol, SolCall};
use eyre::{eyre, Result};

use crate::UnsignedTransaction;

use super::UniswapV4PoolKey;

pub const DEFAULT_UNISWAP_V4_TRADING_VAULT_BUY_GAS_LIMIT: u64 = 350_000;
pub const DEFAULT_UNISWAP_V4_TRADING_VAULT_SELL_GAS_LIMIT: u64 = 450_000;

sol! {
    struct PoolKey {
        address currency0;
        address currency1;
        uint24 fee;
        int24 tickSpacing;
        address hooks;
    }

    function buyV4ExactEthForTokens(
        PoolKey poolKey,
        address tokenOut,
        uint128 minTokensOut,
        uint256 deadline,
        bytes hookData
    ) external payable returns (uint256 tokensReceived);

    function emergencySellV4ExactTokensForEth(
        PoolKey poolKey,
        address tokenIn,
        uint128 amountIn,
        uint128 minEthOut,
        uint256 deadline,
        bytes hookData
    ) external returns (uint256 ethReceived);
}

pub fn encode_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens(
    pool_key: &UniswapV4PoolKey,
    token_out: Address,
    min_tokens_out: U256,
    deadline: U256,
    hook_data: &[u8],
) -> Result<Bytes> {
    Ok(Bytes::from(
        buyV4ExactEthForTokensCall {
            poolKey: pool_key_to_sol(pool_key)?,
            tokenOut: token_out,
            minTokensOut: u256_to_u128(min_tokens_out, "min_tokens_out")?,
            deadline,
            hookData: Bytes::copy_from_slice(hook_data),
        }
        .abi_encode(),
    ))
}

pub fn encode_uniswap_v4_trading_vault_emergency_sell_v4_exact_tokens_for_eth(
    pool_key: &UniswapV4PoolKey,
    token_in: Address,
    amount_in: U256,
    min_eth_out: U256,
    deadline: U256,
    hook_data: &[u8],
) -> Result<Bytes> {
    Ok(Bytes::from(
        emergencySellV4ExactTokensForEthCall {
            poolKey: pool_key_to_sol(pool_key)?,
            tokenIn: token_in,
            amountIn: u256_to_u128(amount_in, "amount_in")?,
            minEthOut: u256_to_u128(min_eth_out, "min_eth_out")?,
            deadline,
            hookData: Bytes::copy_from_slice(hook_data),
        }
        .abi_encode(),
    ))
}

pub fn build_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens(
    vault: Address,
    owner: Address,
    pool_key: &UniswapV4PoolKey,
    token_out: Address,
    amount_in_eth: U256,
    min_tokens_out: U256,
    deadline: U256,
    hook_data: &[u8],
) -> Result<UnsignedTransaction> {
    Ok(UnsignedTransaction {
        from: Some(owner),
        to: Some(vault),
        gas: Some(DEFAULT_UNISWAP_V4_TRADING_VAULT_BUY_GAS_LIMIT),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(encode_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens(
            pool_key,
            token_out,
            min_tokens_out,
            deadline,
            hook_data,
        )?),
        nonce: None,
        ..Default::default()
    })
}

pub fn build_uniswap_v4_trading_vault_emergency_sell_v4_exact_tokens_for_eth(
    vault: Address,
    owner: Address,
    pool_key: &UniswapV4PoolKey,
    token_in: Address,
    amount_in: U256,
    min_eth_out: U256,
    deadline: U256,
    hook_data: &[u8],
) -> Result<UnsignedTransaction> {
    Ok(UnsignedTransaction {
        from: Some(owner),
        to: Some(vault),
        gas: Some(DEFAULT_UNISWAP_V4_TRADING_VAULT_SELL_GAS_LIMIT),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(
            encode_uniswap_v4_trading_vault_emergency_sell_v4_exact_tokens_for_eth(
                pool_key,
                token_in,
                amount_in,
                min_eth_out,
                deadline,
                hook_data,
            )?,
        ),
        nonce: None,
        ..Default::default()
    })
}

fn pool_key_to_sol(pool_key: &UniswapV4PoolKey) -> Result<PoolKey> {
    Ok(PoolKey {
        currency0: pool_key.currency0,
        currency1: pool_key.currency1,
        fee: U24::try_from(pool_key.fee)
            .map_err(|_| eyre!("fee {} exceeds uint24", pool_key.fee))?,
        tickSpacing: I24::try_from(pool_key.tick_spacing)
            .map_err(|_| eyre!("tick_spacing {} exceeds int24", pool_key.tick_spacing))?,
        hooks: pool_key.hooks,
    })
}

fn u256_to_u128(value: U256, label: &str) -> Result<u128> {
    value
        .try_into()
        .map_err(|_| eyre!("{label} exceeds uint128"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::keccak256;

    fn sample_pool_key() -> UniswapV4PoolKey {
        UniswapV4PoolKey {
            currency0: Address::ZERO,
            currency1: Address::with_last_byte(0x11),
            fee: 500,
            tick_spacing: 10,
            hooks: Address::ZERO,
        }
    }

    #[test]
    fn encodes_buy_selector() {
        let data = encode_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens(
            &sample_pool_key(),
            Address::with_last_byte(0x11),
            U256::from(1),
            U256::from(1_800_000_000_u64),
            &[],
        )
        .unwrap();
        let selector =
            keccak256("buyV4ExactEthForTokens((address,address,uint24,int24,address),address,uint128,uint256,bytes)".as_bytes());

        assert_eq!(&data[..4], &selector[..4]);
    }

    #[test]
    fn encodes_sell_selector() {
        let data = encode_uniswap_v4_trading_vault_emergency_sell_v4_exact_tokens_for_eth(
            &sample_pool_key(),
            Address::with_last_byte(0x11),
            U256::from(123),
            U256::from(45),
            U256::from(1_800_000_000_u64),
            &[],
        )
        .unwrap();
        let selector =
            keccak256("emergencySellV4ExactTokensForEth((address,address,uint24,int24,address),address,uint128,uint128,uint256,bytes)".as_bytes());

        assert_eq!(&data[..4], &selector[..4]);
    }

    #[test]
    fn builds_buy_transaction_to_vault() {
        let vault = Address::with_last_byte(0xaa);
        let owner = Address::with_last_byte(0xbb);
        let token = Address::with_last_byte(0xcc);
        let tx = build_uniswap_v4_trading_vault_buy_v4_exact_eth_for_tokens(
            vault,
            owner,
            &sample_pool_key(),
            token,
            U256::from(1_000_000_000_000_000_000u128),
            U256::from(1),
            U256::from(1_800_000_000_u64),
            &[],
        )
        .unwrap();

        assert_eq!(tx.from, Some(owner));
        assert_eq!(tx.to, Some(vault));
        assert_eq!(tx.value, Some(U256::from(1_000_000_000_000_000_000u128)));
        assert_eq!(tx.gas, Some(DEFAULT_UNISWAP_V4_TRADING_VAULT_BUY_GAS_LIMIT));
        assert!(tx.data.expect("calldata").len() > 4);
    }
}
