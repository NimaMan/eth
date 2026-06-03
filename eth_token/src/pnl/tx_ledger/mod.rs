use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

pub mod bridge;
pub mod conservation;
pub mod model;
pub mod native;
pub mod raw;
pub mod reconcile;

pub use bridge::{bridge_movements, detect_weth_bridges, WethBridge, WethBridgeKind};
pub use conservation::{conservation_summary, ConservationSummary};
pub use model::{
    AssetFamily, MovementSource, RawDelta, RawSourceKind, SignedRawAmount, TxAsset,
    TxLedgerContext, TxMovement, WETH_ADDRESS,
};
pub use native::{is_balance_moving_internal_eth, movement_from_internal_trace};
pub use raw::movements_from_processed_transaction;
pub use reconcile::{
    reconcile_movements, AddressReconciliationKind, AddressReconciliationSummary,
    ReconciledTxLedger,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TxLedger {
    pub tx_hash: B256,
    pub block_number: u64,
    pub tx_index: u64,
    pub context: TxLedgerContext,
    pub movements: Vec<TxMovement>,
    pub weth_bridges: Vec<WethBridge>,
}

impl TxLedger {
    pub fn from_processed_transaction(
        context: TxLedgerContext,
        transaction: &ProcessedTransaction,
    ) -> Self {
        let mut movements = movements_from_processed_transaction(transaction);
        let weth_bridges = detect_weth_bridges(transaction);
        movements.extend(bridge_movements(&weth_bridges));

        Self {
            tx_hash: transaction.hash,
            block_number: transaction.block_number,
            tx_index: transaction.tx_index,
            context,
            movements,
            weth_bridges,
        }
    }

    pub fn reconcile(&self) -> ReconciledTxLedger {
        reconcile_movements(self.context, &self.movements)
    }

    pub fn conservation(&self) -> ConservationSummary {
        conservation_summary(self.context, &self.movements)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, Address, U256};
    use tx_processor::tx_processor::data_models::tx_models::ETHTransfer;
    use tx_processor::tx_processor::data_models::{
        DepositEvent, ERC20TransferEvent, InternalTransaction, ProcessedTransaction,
    };

    use super::*;

    const USER: Address = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    const ROUTER: Address = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    const IMPLEMENTATION: Address = address!("cccccccccccccccccccccccccccccccccccccccc");
    const POOL: Address = address!("dddddddddddddddddddddddddddddddddddddddd");
    const TOKEN: Address = address!("1111111111111111111111111111111111111111");
    const TWO_ETH: u128 = 2_000_000_000_000_000_000;
    const TOKEN_AMOUNT: u64 = 50_000;

    #[test]
    fn banana_gun_shaped_tx_reconciles_router_as_denom_pass_through() {
        let mut tx = tx();
        tx.eth_transfers.push(ETHTransfer {
            from_address: USER,
            to_address: ROUTER,
            amount: U256::from(TWO_ETH),
        });
        tx.internal_transactions.push(InternalTransaction {
            from_address: USER,
            to_address: Some(ROUTER),
            value: U256::from(TWO_ETH),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("CALL".to_string()),
            depth: 0,
            error: None,
        });
        tx.internal_transactions.push(InternalTransaction {
            from_address: ROUTER,
            to_address: Some(IMPLEMENTATION),
            value: U256::from(TWO_ETH),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("DELEGATECALL".to_string()),
            depth: 1,
            error: None,
        });
        tx.internal_transactions.push(InternalTransaction {
            from_address: ROUTER,
            to_address: Some(WETH_ADDRESS),
            value: U256::from(TWO_ETH),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("CALL".to_string()),
            depth: 1,
            error: None,
        });
        tx.deposit_events.push(DepositEvent {
            id: None,
            token_address: None,
            withdrawal_address: None,
            amount: Some(U256::from(TWO_ETH)),
            unlock_time: None,
            pair_address: Some(WETH_ADDRESS),
            sender: Some(ROUTER),
            log_index: Some(1),
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: WETH_ADDRESS,
            from_address: ROUTER,
            to_address: POOL,
            amount: U256::from(TWO_ETH),
            log_index: 2,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: TOKEN,
            from_address: POOL,
            to_address: USER,
            amount: U256::from(TOKEN_AMOUNT),
            log_index: 3,
        });

        let ledger =
            TxLedger::from_processed_transaction(TxLedgerContext::new(TOKEN, WETH_ADDRESS), &tx);
        let reconciled = ledger.reconcile();

        assert!(reconciled.address(&IMPLEMENTATION).is_none());
        assert_eq!(ledger.weth_bridges.len(), 1);
        assert!(ledger
            .conservation()
            .family_is_conserved(AssetFamily::Denom));
        assert!(ledger
            .conservation()
            .family_is_conserved(AssetFamily::Token));

        let router = reconciled.address(&ROUTER).expect("router summary");
        assert!(router.is_pass_through());
        assert!(router.is_family_net_zero(AssetFamily::Denom));
        assert_eq!(
            router
                .asset_delta(TxAsset::native_eth())
                .expect("router native")
                .net()
                .raw_string(),
            "0"
        );
        assert_eq!(
            router
                .asset_delta(TxAsset::erc20(WETH_ADDRESS))
                .expect("router weth")
                .net()
                .raw_string(),
            "0"
        );

        let user = reconciled.address(&USER).expect("user summary");
        assert_eq!(
            user.family_delta(AssetFamily::Denom)
                .expect("user denom")
                .net()
                .raw_string(),
            format!("-{TWO_ETH}")
        );
        assert_eq!(
            user.family_delta(AssetFamily::Token)
                .expect("user token")
                .net()
                .raw_string(),
            TOKEN_AMOUNT.to_string()
        );

        let pool = reconciled.address(&POOL).expect("pool summary");
        let pool_weth = pool
            .asset_delta(TxAsset::erc20(WETH_ADDRESS))
            .expect("pool weth");
        assert_eq!(pool_weth.incoming, U256::from(TWO_ETH));
        assert_eq!(pool_weth.outgoing, U256::ZERO);
        assert_eq!(
            pool.family_delta(AssetFamily::Token)
                .expect("pool token")
                .net()
                .raw_string(),
            format!("-{TOKEN_AMOUNT}")
        );
    }

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0101010101010101010101010101010101010101010101010101010101010101"),
            100,
            1_700,
            1,
            USER,
            Some(ROUTER),
            U256::from(TWO_ETH),
            true,
            0,
            2,
            Vec::new(),
        )
    }
}
