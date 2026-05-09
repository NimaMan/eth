use alloy_primitives::{address, Address, U256};
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::pools::UniswapV2Pool;

const TRANSFER_FROM_FAILED: &str = "TransferHelper: TRANSFER_FROM_FAILED";
const BUY_RECEIVED_ZERO_TOKENS: &str = "buyer received zero tokens";
const UNISWAP_V2_ROUTER: Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
const UNISWAP_UNIVERSAL_ROUTER: Address = address!("3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD");
const UNISWAP_UNIVERSAL_ROUTER_LEGACY: Address =
    address!("Ef1c6E67703c7BD7107eed8303Fbe6EC2554BF6B");
const UNISWAP_UNIVERSAL_ROUTER_V2: Address = address!("4C82D1fBFe28C977cBB58D8C7FF8FCF9F70a2cCA");
const PERMIT2: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TradingFailureClass {
    BuyReceivedZeroTokens,
    ObservedSellViaUniversalRouterPermit2,
    ObservedSellViaExternalRouter,
    ObservedSellViaClassicRouter,
    ObservedBuyViaUniversalRouterPermit2,
    ObservedBuyViaExternalRouter,
    CurrentTxUsesUniversalRouterPermit2,
    CurrentTxUsesExternalRouter,
    FreshHolderTransferFromFailed,
}

impl TradingFailureClass {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::BuyReceivedZeroTokens => "buy_received_zero_tokens",
            Self::ObservedSellViaUniversalRouterPermit2 => {
                "observed_sell_via_universal_router_permit2"
            }
            Self::ObservedSellViaExternalRouter => "observed_sell_via_external_router",
            Self::ObservedSellViaClassicRouter => "observed_sell_via_classic_router",
            Self::ObservedBuyViaUniversalRouterPermit2 => {
                "observed_buy_via_universal_router_permit2"
            }
            Self::ObservedBuyViaExternalRouter => "observed_buy_via_external_router",
            Self::CurrentTxUsesUniversalRouterPermit2 => "current_tx_uses_universal_router_permit2",
            Self::CurrentTxUsesExternalRouter => "current_tx_uses_external_router",
            Self::FreshHolderTransferFromFailed => "fresh_holder_transfer_from_failed",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::BuyReceivedZeroTokens => {
                "simulated buy transaction succeeded but delivered zero tokens to the buyer"
            }
            Self::ObservedSellViaUniversalRouterPermit2 => {
                "chain sell succeeded through Universal Router or Permit2; classic V2 simulator path is not route-equivalent"
            }
            Self::ObservedSellViaExternalRouter => {
                "chain sell succeeded through a non-classic router; simulator needs actual route parity before calling it a honeypot"
            }
            Self::ObservedSellViaClassicRouter => {
                "chain sell succeeded for the transaction sender; simulated fresh-holder sell failure is likely state, holder, or buy-route dependent"
            }
            Self::ObservedBuyViaUniversalRouterPermit2 => {
                "current chain buy used Universal Router or Permit2; the later sell check may depend on that route"
            }
            Self::ObservedBuyViaExternalRouter => {
                "current chain buy used a non-classic router; the later sell check may depend on that entrypoint"
            }
            Self::CurrentTxUsesUniversalRouterPermit2 => {
                "current transaction uses Universal Router or Permit2 while the simulator uses classic V2 router"
            }
            Self::CurrentTxUsesExternalRouter => {
                "current transaction uses a non-classic router while the simulator uses classic V2 router"
            }
            Self::FreshHolderTransferFromFailed => {
                "fresh simulated holder bought successfully but token transferFrom failed on sell"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedSwapDirection {
    Buy,
    Sell,
    Mixed,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransactionRoute {
    ClassicV2Router,
    UniversalRouterPermit2,
    ExternalRouter,
    DirectOrUnknown,
}

pub(crate) fn classify_v2_trading_failure(
    pool: &UniswapV2Pool,
    tx: &ProcessedTransaction,
    failure_reason: &str,
) -> Option<TradingFailureClass> {
    if failure_reason.contains(BUY_RECEIVED_ZERO_TOKENS) {
        return Some(TradingFailureClass::BuyReceivedZeroTokens);
    }
    if !is_transfer_from_failed(failure_reason) {
        return None;
    }

    let route = transaction_route(tx);
    let direction = observed_pool_swap_direction(pool, tx);

    match direction {
        ObservedSwapDirection::Sell | ObservedSwapDirection::Mixed => match route {
            TransactionRoute::UniversalRouterPermit2 => {
                Some(TradingFailureClass::ObservedSellViaUniversalRouterPermit2)
            }
            TransactionRoute::ExternalRouter => {
                Some(TradingFailureClass::ObservedSellViaExternalRouter)
            }
            TransactionRoute::ClassicV2Router | TransactionRoute::DirectOrUnknown => {
                Some(TradingFailureClass::ObservedSellViaClassicRouter)
            }
        },
        ObservedSwapDirection::Buy => match route {
            TransactionRoute::UniversalRouterPermit2 => {
                Some(TradingFailureClass::ObservedBuyViaUniversalRouterPermit2)
            }
            TransactionRoute::ExternalRouter => {
                Some(TradingFailureClass::ObservedBuyViaExternalRouter)
            }
            TransactionRoute::ClassicV2Router | TransactionRoute::DirectOrUnknown => {
                Some(TradingFailureClass::FreshHolderTransferFromFailed)
            }
        },
        ObservedSwapDirection::None => match route {
            TransactionRoute::UniversalRouterPermit2 => {
                Some(TradingFailureClass::CurrentTxUsesUniversalRouterPermit2)
            }
            TransactionRoute::ExternalRouter => {
                Some(TradingFailureClass::CurrentTxUsesExternalRouter)
            }
            TransactionRoute::ClassicV2Router | TransactionRoute::DirectOrUnknown => {
                Some(TradingFailureClass::FreshHolderTransferFromFailed)
            }
        },
    }
}

fn is_transfer_from_failed(reason: &str) -> bool {
    reason.contains(TRANSFER_FROM_FAILED)
}

fn transaction_route(tx: &ProcessedTransaction) -> TransactionRoute {
    if tx_uses_universal_router_or_permit2(tx) {
        return TransactionRoute::UniversalRouterPermit2;
    }

    match tx.to_address {
        Some(address) if address == UNISWAP_V2_ROUTER => TransactionRoute::ClassicV2Router,
        Some(address) if tx.input.len() >= 4 && address != pool_or_token_direct_target(tx) => {
            TransactionRoute::ExternalRouter
        }
        Some(_) | None => TransactionRoute::DirectOrUnknown,
    }
}

fn tx_uses_universal_router_or_permit2(tx: &ProcessedTransaction) -> bool {
    tx.to_address.is_some_and(|address| {
        matches!(
            address,
            UNISWAP_UNIVERSAL_ROUTER
                | UNISWAP_UNIVERSAL_ROUTER_LEGACY
                | UNISWAP_UNIVERSAL_ROUTER_V2
                | PERMIT2
        )
    }) || tx.unique_addresses.contains(&PERMIT2)
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| event.spender == PERMIT2)
        || !tx.permit2_events.is_empty()
}

fn pool_or_token_direct_target(tx: &ProcessedTransaction) -> Address {
    if let Some(transfer) = tx.erc20_transfers.first() {
        return transfer.token_address;
    }
    if let Some(swap) = tx.uniswap_v2_swaps.first() {
        return swap.pair_address;
    }
    Address::ZERO
}

fn observed_pool_swap_direction(
    pool: &UniswapV2Pool,
    tx: &ProcessedTransaction,
) -> ObservedSwapDirection {
    let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(true);
    let mut saw_buy = false;
    let mut saw_sell = false;

    for swap in &tx.uniswap_v2_swaps {
        if !same_address(swap.pair_address, &pool.base.identity.pool_address) {
            continue;
        }

        let is_buy = if token1_is_denom {
            swap.amount0_out > U256::ZERO && swap.amount1_in > U256::ZERO
        } else {
            swap.amount1_out > U256::ZERO && swap.amount0_in > U256::ZERO
        };
        let is_sell = if token1_is_denom {
            swap.amount0_in > U256::ZERO && swap.amount1_out > U256::ZERO
        } else {
            swap.amount1_in > U256::ZERO && swap.amount0_out > U256::ZERO
        };

        saw_buy |= is_buy;
        saw_sell |= is_sell;
    }

    match (saw_buy, saw_sell) {
        (true, true) => ObservedSwapDirection::Mixed,
        (true, false) => ObservedSwapDirection::Buy,
        (false, true) => ObservedSwapDirection::Sell,
        (false, false) => ObservedSwapDirection::None,
    }
}

fn same_address(address: Address, other: &str) -> bool {
    other
        .parse::<Address>()
        .is_ok_and(|other_address| address == other_address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use tx_processor::tx_processor::data_models::receipt_models::UniswapV2SwapEvent;

    use crate::pools::{BasePoolConfig, UniswapV2Pool};

    fn pool(token1_is_denom: bool) -> UniswapV2Pool {
        UniswapV2Pool::new(
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "0x3333333333333333333333333333333333333333",
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(token1_is_denom),
                history_limit: 10,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: 0.01,
            },
            std::iter::empty::<&str>(),
        )
    }

    fn tx(to_address: Option<Address>) -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0101010101010101010101010101010101010101010101010101010101010101"),
            100,
            1_700,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            to_address,
            U256::ZERO,
            true,
            0,
            2,
            vec![0x12, 0x34, 0x56, 0x78],
        )
    }

    #[test]
    fn classifies_observed_universal_router_sell_as_route_mismatch() {
        let pool = pool(true);
        let mut tx = tx(Some(UNISWAP_UNIVERSAL_ROUTER_V2));
        tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
            pair_address: address!("1111111111111111111111111111111111111111"),
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            to: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            amount0_in: U256::from(10),
            amount1_in: U256::ZERO,
            amount0_out: U256::ZERO,
            amount1_out: U256::from(1),
            log_index: 1,
        });

        assert_eq!(
            classify_v2_trading_failure(&pool, &tx, TRANSFER_FROM_FAILED),
            Some(TradingFailureClass::ObservedSellViaUniversalRouterPermit2)
        );
    }

    #[test]
    fn classifies_observed_classic_router_sell_as_sender_state_dependent() {
        let pool = pool(true);
        let mut tx = tx(Some(UNISWAP_V2_ROUTER));
        tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
            pair_address: address!("1111111111111111111111111111111111111111"),
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            to: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            amount0_in: U256::from(10),
            amount1_in: U256::ZERO,
            amount0_out: U256::ZERO,
            amount1_out: U256::from(1),
            log_index: 1,
        });

        assert_eq!(
            classify_v2_trading_failure(&pool, &tx, TRANSFER_FROM_FAILED),
            Some(TradingFailureClass::ObservedSellViaClassicRouter)
        );
    }

    #[test]
    fn classifies_zero_output_buy() {
        assert_eq!(
            classify_v2_trading_failure(
                &pool(true),
                &tx(Some(UNISWAP_V2_ROUTER)),
                "Buy transaction succeeded but buyer received zero tokens (denom_spent=10)",
            ),
            Some(TradingFailureClass::BuyReceivedZeroTokens)
        );
    }
}
