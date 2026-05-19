use crate::UnsignedTransaction;
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};

pub const DEFAULT_UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT: u64 = 300_000;
pub const DEFAULT_UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT: u64 = 300_000;

sol! {
    function buyV2ExactEthForTokens(address token, uint256 minTokensOut, uint256 deadline)
        external
        payable
        returns (uint256 tokensReceived);

    function emergencySellV2ExactTokensForEth(
        address token,
        uint256 amountIn,
        uint256 minEthOut,
        uint256 deadline
    ) external returns (uint256 ethReceived);
}

pub fn encode_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens(
    token: Address,
    min_tokens_out: U256,
    deadline: u64,
) -> Bytes {
    Bytes::from(
        buyV2ExactEthForTokensCall {
            token,
            minTokensOut: min_tokens_out,
            deadline: U256::from(deadline),
        }
        .abi_encode(),
    )
}

pub fn encode_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
    token: Address,
    amount_in: U256,
    min_eth_out: U256,
    deadline: u64,
) -> Bytes {
    Bytes::from(
        emergencySellV2ExactTokensForEthCall {
            token,
            amountIn: amount_in,
            minEthOut: min_eth_out,
            deadline: U256::from(deadline),
        }
        .abi_encode(),
    )
}

pub fn build_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens(
    vault: Address,
    owner: Address,
    token: Address,
    amount_in_eth: U256,
    min_tokens_out: U256,
    deadline: u64,
) -> UnsignedTransaction {
    UnsignedTransaction {
        from: Some(owner),
        to: Some(vault),
        gas: Some(DEFAULT_UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(encode_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens(
            token,
            min_tokens_out,
            deadline,
        )),
        nonce: None,
        ..Default::default()
    }
}

pub fn build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
    vault: Address,
    owner: Address,
    token: Address,
    amount_in: U256,
    min_eth_out: U256,
    deadline: u64,
) -> UnsignedTransaction {
    UnsignedTransaction {
        from: Some(owner),
        to: Some(vault),
        gas: Some(DEFAULT_UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(
            encode_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
                token,
                amount_in,
                min_eth_out,
                deadline,
            ),
        ),
        nonce: None,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::keccak256;

    #[test]
    fn encodes_emergency_sell_selector_and_fields() {
        let token = Address::with_last_byte(0x11);
        let data = encode_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
            token,
            U256::from(123),
            U256::from(45),
            1_800_000_000,
        );
        let selector = keccak256(
            "emergencySellV2ExactTokensForEth(address,uint256,uint256,uint256)".as_bytes(),
        );

        assert_eq!(&data[..4], &selector[..4]);
        assert_eq!(data.len(), 4 + 32 * 4);
    }

    #[test]
    fn builds_buy_transaction_to_vault() {
        let vault = Address::with_last_byte(0xaa);
        let owner = Address::with_last_byte(0xbb);
        let token = Address::with_last_byte(0xcc);
        let tx = build_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens(
            vault,
            owner,
            token,
            U256::from(1_000_000_000_000_000_000u128),
            U256::from(1),
            1_800_000_000,
        );

        assert_eq!(tx.from, Some(owner));
        assert_eq!(tx.to, Some(vault));
        assert_eq!(tx.value, Some(U256::from(1_000_000_000_000_000_000u128)));
        assert_eq!(tx.gas, Some(DEFAULT_UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT));
        assert!(tx.data.expect("calldata").len() > 4);
    }
}
