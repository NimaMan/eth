use crate::tx_builders::protocols::v3_swap_router::{
    self, ExactInputSingleAbi, ExactInputSingleRequest, WETH_ADDRESS,
};
use crate::tx_builders::PermitData;
use crate::UnsignedTransaction;
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall, SolValue};

const COMMAND_V3_SWAP_EXACT_IN: u8 = 0x00;
const COMMAND_UNWRAP_WETH: u8 = 0x0c;
const UNIVERSAL_ROUTER_ADDRESS_THIS: Address = address!("0000000000000000000000000000000000000002");
pub const DEFAULT_ROUTER: Address = address!("E592427A0AEce92De3Edee1F18E0157C05861564");

sol! {
    function execute(bytes commands, bytes[] inputs, uint256 deadline);
}

fn router_address_v3() -> Address {
    DEFAULT_ROUTER
}

fn weth_address() -> Address {
    WETH_ADDRESS
}

#[derive(Debug, Clone)]
pub struct UniversalRouterV3ExactInputRequest {
    pub universal_router: Address,
    pub caller: Address,
    pub recipient: Address,
    pub token_in: Address,
    pub token_out: Address,
    pub fee: u32,
    pub amount_in: U256,
    pub min_amount_out: U256,
    pub deadline: U256,
    pub payer_is_user: bool,
    pub unwrap_weth_to: Option<Address>,
}

pub fn build_universal_router_v3_exact_input_tx(
    request: &UniversalRouterV3ExactInputRequest,
) -> eyre::Result<UnsignedTransaction> {
    let path = encode_v3_path(request.token_in, request.fee, request.token_out)?;
    let mut commands = vec![COMMAND_V3_SWAP_EXACT_IN];
    let mut inputs = Vec::new();
    let swap_recipient = if request.unwrap_weth_to.is_some() {
        UNIVERSAL_ROUTER_ADDRESS_THIS
    } else {
        request.recipient
    };
    let input = (
        swap_recipient,
        request.amount_in,
        request.min_amount_out,
        Bytes::from(path),
        request.payer_is_user,
        Bytes::new(),
    )
        .abi_encode_params();
    inputs.push(Bytes::from(input));

    if let Some(unwrap_recipient) = request.unwrap_weth_to {
        commands.push(COMMAND_UNWRAP_WETH);
        inputs.push(Bytes::from(
            (unwrap_recipient, request.min_amount_out).abi_encode_params(),
        ));
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
        value: Some(U256::ZERO),
        data: Some(Bytes::from(calldata)),
        nonce: None,
        ..Default::default()
    })
}

fn encode_v3_path(token_in: Address, fee: u32, token_out: Address) -> eyre::Result<Vec<u8>> {
    if fee > 0x00ff_ffff {
        eyre::bail!("Uniswap V3 fee {fee} exceeds uint24");
    }
    let mut path = Vec::with_capacity(43);
    path.extend_from_slice(token_in.as_slice());
    let fee_bytes = fee.to_be_bytes();
    path.extend_from_slice(&fee_bytes[1..]);
    path.extend_from_slice(token_out.as_slice());
    Ok(path)
}

fn build_exact_input_single(
    router: Address,
    caller: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    recipient: Address,
    amount_out_minimum: U256,
    deadline: U256,
    value: U256,
) -> UnsignedTransaction {
    v3_swap_router::build_exact_input_single_tx(ExactInputSingleRequest {
        router,
        caller,
        token_in,
        token_out,
        fee: fee_tier,
        recipient,
        deadline,
        amount_in,
        amount_out_minimum,
        sqrt_price_limit_x96: U256::ZERO,
        value,
        abi: ExactInputSingleAbi::WithDeadline,
    })
}

/// Build a Uniswap V3 buy swap (ETH -> token) via SwapRouter exactInputSingle.
pub fn build_buy_swap_v3(
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_buy_swap_v3_with_router(
        router_address_v3(),
        buyer,
        token_out,
        amount_in_eth,
        fee_tier,
        deadline,
    )
}

pub fn build_buy_swap_v3_with_router(
    router: Address,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        router,
        buyer,
        weth_address(),
        token_out,
        amount_in_eth,
        fee_tier,
        buyer,
        U256::ZERO, // accept-any for testing
        U256::from(deadline),
        amount_in_eth,
    )
}

/// Build a Uniswap V3 buy swap with explicit amountOutMinimum.
pub fn build_buy_swap_v3_with_min_out(
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_buy_swap_v3_with_min_out_router(
        router_address_v3(),
        buyer,
        token_out,
        amount_in_eth,
        fee_tier,
        amount_out_min,
        deadline,
    )
}

pub fn build_buy_swap_v3_with_min_out_router(
    router: Address,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        router,
        buyer,
        weth_address(),
        token_out,
        amount_in_eth,
        fee_tier,
        buyer,
        amount_out_min,
        U256::from(deadline),
        amount_in_eth,
    )
}

/// Build approve(tx) for V3 SwapRouter as spender.
pub fn build_approve_v3(owner: Address, token: Address, amount: U256) -> UnsignedTransaction {
    build_approve_v3_for_router(router_address_v3(), owner, token, amount)
}

pub fn build_approve_v3_for_router(
    router: Address,
    owner: Address,
    token: Address,
    amount: U256,
) -> UnsignedTransaction {
    v3_swap_router::build_approve_for_router(router, owner, token, amount)
}

/// Build a Uniswap V3 sell swap (Token -> WETH) via SwapRouter exactInputSingle.
pub fn build_sell_swap_v3(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_sell_swap_v3_with_router(
        router_address_v3(),
        seller,
        token_in,
        amount_in_tokens,
        fee_tier,
        deadline,
    )
}

pub fn build_sell_swap_v3_with_router(
    router: Address,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        router,
        seller,
        token_in,
        weth_address(),
        amount_in_tokens,
        fee_tier,
        seller,
        U256::ZERO, // accept-any for testing
        U256::from(deadline),
        U256::ZERO,
    )
}

/// Build a Uniswap V3 sell swap with explicit amountOutMinimum.
pub fn build_sell_swap_v3_with_min_out(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_sell_swap_v3_with_min_out_router(
        router_address_v3(),
        seller,
        token_in,
        amount_in_tokens,
        fee_tier,
        amount_out_min,
        deadline,
    )
}

pub fn build_sell_swap_v3_with_min_out_router(
    router: Address,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        router,
        seller,
        token_in,
        weth_address(),
        amount_in_tokens,
        fee_tier,
        seller,
        amount_out_min,
        U256::from(deadline),
        U256::ZERO,
    )
}

/// Build a Uniswap V3 token -> token swap (exactInputSingle).
pub fn build_token_to_token_swap_v3(
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_token_to_token_swap_v3_with_router(
        router_address_v3(),
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        deadline,
    )
}

pub fn build_token_to_token_swap_v3_with_router(
    router: Address,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        router,
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        trader,
        U256::ZERO,
        U256::from(deadline),
        U256::ZERO,
    )
}

/// Build a Uniswap V3 token -> token swap with explicit amountOutMinimum.
pub fn build_token_to_token_swap_v3_with_min_out(
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_token_to_token_swap_v3_with_min_out_router(
        router_address_v3(),
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        amount_out_min,
        deadline,
    )
}

pub fn build_token_to_token_swap_v3_with_min_out_router(
    router: Address,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        router,
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        trader,
        amount_out_min,
        U256::from(deadline),
        U256::ZERO,
    )
}

/// Build a Uniswap V3 sell swap that attempts to include a selfPermit via multicall.
/// NOTE: Placeholder implementation currently falls back to a standard sell swap.
/// Proper selfPermit + multicall encoding will be added in a subsequent pass.
pub fn build_sell_with_self_permit_v3(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    _permit: PermitData,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    // Fallback: build standard sell (no permit bundling yet)
    build_sell_swap_v3(seller, token_in, amount_in_tokens, fee_tier, 0, deadline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_universal_router_v3_exact_input() {
        let caller = Address::with_last_byte(1);
        let token_in = Address::with_last_byte(2);
        let token_out = Address::with_last_byte(3);
        let router = Address::with_last_byte(4);

        let tx = build_universal_router_v3_exact_input_tx(&UniversalRouterV3ExactInputRequest {
            universal_router: router,
            caller,
            recipient: caller,
            token_in,
            token_out,
            fee: 3000,
            amount_in: U256::from(1000u64),
            min_amount_out: U256::ZERO,
            deadline: U256::from(123u64),
            payer_is_user: true,
            unwrap_weth_to: None,
        })
        .unwrap();

        let data = tx.data.unwrap();
        assert_eq!(tx.from, Some(caller));
        assert_eq!(tx.to, Some(router));
        assert_eq!(tx.value, Some(U256::ZERO));
        assert_eq!(&data[..4], executeCall::SELECTOR);
        assert!(data
            .windows(1)
            .any(|window| window == [COMMAND_V3_SWAP_EXACT_IN]));
    }

    #[test]
    fn rejects_universal_router_v3_fee_outside_uint24() {
        let err = build_universal_router_v3_exact_input_tx(&UniversalRouterV3ExactInputRequest {
            universal_router: Address::with_last_byte(4),
            caller: Address::with_last_byte(1),
            recipient: Address::with_last_byte(1),
            token_in: Address::with_last_byte(2),
            token_out: Address::with_last_byte(3),
            fee: 0x0100_0000,
            amount_in: U256::from(1000u64),
            min_amount_out: U256::ZERO,
            deadline: U256::from(123u64),
            payer_is_user: true,
            unwrap_weth_to: None,
        })
        .unwrap_err();

        assert!(err.to_string().contains("exceeds uint24"));
    }

    #[test]
    fn encodes_universal_router_v3_unwrap_sequence() {
        let caller = Address::with_last_byte(1);
        let token_in = Address::with_last_byte(2);
        let weth = weth_address();
        let router = Address::with_last_byte(4);

        let tx = build_universal_router_v3_exact_input_tx(&UniversalRouterV3ExactInputRequest {
            universal_router: router,
            caller,
            recipient: caller,
            token_in,
            token_out: weth,
            fee: 3000,
            amount_in: U256::from(1000u64),
            min_amount_out: U256::ZERO,
            deadline: U256::from(123u64),
            payer_is_user: true,
            unwrap_weth_to: Some(caller),
        })
        .unwrap();

        let data = tx.data.unwrap();
        assert_eq!(&data[..4], executeCall::SELECTOR);
        assert!(data
            .windows(2)
            .any(|window| { window == [COMMAND_V3_SWAP_EXACT_IN, COMMAND_UNWRAP_WETH] }));
        assert!(data
            .windows(32)
            .any(|window| { window == UNIVERSAL_ROUTER_ADDRESS_THIS.into_word().as_slice() }));
    }
}
