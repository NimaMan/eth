//! Pool-derived network update extraction.

use alloy_primitives::{Address, B256, U256};
use tx_processor::ProcessedTransaction;

use crate::network::{
    ingest::transaction::{
        address_string, edge_with_evidence, NetworkIngestBatch, NodeLabelIngest,
        TransactionIngestContext,
    },
    model::{
        NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource, NetworkLabelKind,
        NetworkLabelSource, NetworkNodeId, NetworkPoolId,
    },
};

pub const UNISWAP_V2_PROTOCOL: &str = "uniswap_v2";
pub const UNISWAP_V3_PROTOCOL: &str = "uniswap_v3";
pub const UNISWAP_V4_PROTOCOL: &str = "uniswap_v4";

/// Extract pool-related relationships observed in a processed transaction.
pub fn extract_pool_updates(
    tx: &ProcessedTransaction,
    tracked_token_address: impl AsRef<str>,
) -> NetworkIngestBatch {
    let mut batch = NetworkIngestBatch::default();
    if !tx.status {
        return batch;
    }

    let context = TransactionIngestContext::from_transaction(tx);
    let tracked_token_address = tracked_token_address.as_ref().trim().to_ascii_lowercase();
    let token_node = NetworkNodeId::token(&tracked_token_address);

    for event in &tx.uniswap_v2_pair_created_events {
        if !pair_contains_tracked_token(event.token0, event.token1, &tracked_token_address) {
            continue;
        }
        let pool_node = NetworkNodeId::pool_address(address_string(&event.pair_address));
        push_pool_label(
            &mut batch,
            &context,
            pool_node.clone(),
            Some(event.log_index),
        );
        let mut edge = pool_edge(
            &context,
            token_node.clone(),
            pool_node,
            NetworkEdgeKind::PoolCreation,
            NetworkEvidenceSource::PoolState,
            Some(event.log_index),
            "uniswap_v2_pair_created",
            "tracked token Uniswap V2 pair created",
        );
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V2_PROTOCOL.to_string());
        edge.attributes
            .insert("token0".to_string(), address_string(&event.token0));
        edge.attributes
            .insert("token1".to_string(), address_string(&event.token1));
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v2_swaps {
        let pool_node = NetworkNodeId::pool_address(address_string(&event.pair_address));
        let mut edge = pool_edge(
            &context,
            NetworkNodeId::address(address_string(&event.sender)),
            pool_node,
            NetworkEdgeKind::PoolTrade,
            NetworkEvidenceSource::PoolTrade,
            Some(event.log_index),
            "uniswap_v2_swap",
            "Uniswap V2 swap involving pool",
        );
        add_v2_amount_attrs(
            &mut edge,
            event.amount0_in,
            event.amount1_in,
            event.amount0_out,
            event.amount1_out,
        );
        edge.attributes
            .insert("recipient".to_string(), address_string(&event.to));
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V2_PROTOCOL.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v2_mints {
        let mut edge = pool_edge(
            &context,
            NetworkNodeId::address(address_string(&event.sender)),
            NetworkNodeId::pool_address(address_string(&event.pair_address)),
            NetworkEdgeKind::LiquidityEvent,
            NetworkEvidenceSource::PoolState,
            Some(event.log_index),
            "uniswap_v2_mint",
            "Uniswap V2 liquidity minted",
        );
        edge.attributes
            .insert("amount0".to_string(), event.amount0.to_string());
        edge.attributes
            .insert("amount1".to_string(), event.amount1.to_string());
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V2_PROTOCOL.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v2_burns {
        let mut edge = pool_edge(
            &context,
            NetworkNodeId::address(address_string(&event.sender)),
            NetworkNodeId::pool_address(address_string(&event.pair_address)),
            NetworkEdgeKind::LiquidityEvent,
            NetworkEvidenceSource::PoolState,
            Some(event.log_index),
            "uniswap_v2_burn",
            "Uniswap V2 liquidity burned",
        );
        edge.attributes
            .insert("amount0".to_string(), event.amount0.to_string());
        edge.attributes
            .insert("amount1".to_string(), event.amount1.to_string());
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V2_PROTOCOL.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v3_pools {
        if !pair_contains_tracked_token(event.token0, event.token1, &tracked_token_address) {
            continue;
        }
        let pool_node = NetworkNodeId::pool_address(address_string(&event.pool));
        push_pool_label(
            &mut batch,
            &context,
            pool_node.clone(),
            Some(event.log_index),
        );
        let mut edge = pool_edge(
            &context,
            token_node.clone(),
            pool_node,
            NetworkEdgeKind::PoolCreation,
            NetworkEvidenceSource::PoolState,
            Some(event.log_index),
            "uniswap_v3_pool_created",
            "tracked token Uniswap V3 pool created",
        );
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V3_PROTOCOL.to_string());
        edge.attributes
            .insert("fee".to_string(), event.fee.to_string());
        edge.attributes
            .insert("tick_spacing".to_string(), event.tick_spacing.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v3_swaps {
        let mut edge = pool_edge(
            &context,
            NetworkNodeId::address(address_string(&event.sender)),
            NetworkNodeId::pool_address(address_string(&event.pool_address)),
            NetworkEdgeKind::PoolTrade,
            NetworkEvidenceSource::PoolTrade,
            Some(event.log_index),
            "uniswap_v3_swap",
            "Uniswap V3 swap involving pool",
        );
        edge.attributes
            .insert("recipient".to_string(), address_string(&event.recipient));
        edge.attributes
            .insert("amount0".to_string(), event.amount0.to_string());
        edge.attributes
            .insert("amount1".to_string(), event.amount1.to_string());
        edge.attributes
            .insert("tick".to_string(), event.tick.to_string());
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V3_PROTOCOL.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v3_mints {
        push_v3_liquidity_edge(
            &mut batch,
            &context,
            event.owner,
            event.pool_address,
            Some(event.log_index),
            "uniswap_v3_mint",
            event.amount,
            event.amount0,
            event.amount1,
        );
    }

    for event in &tx.uniswap_v3_burns {
        push_v3_liquidity_edge(
            &mut batch,
            &context,
            event.owner,
            event.pool_address,
            Some(event.log_index),
            "uniswap_v3_burn",
            event.amount,
            event.amount0,
            event.amount1,
        );
    }

    for event in &tx.uniswap_v3_increases {
        push_v3_liquidity_edge(
            &mut batch,
            &context,
            context_from_address(&context),
            event.pool_address,
            Some(event.log_index),
            "uniswap_v3_increase_liquidity",
            event.liquidity,
            event.amount0,
            event.amount1,
        );
    }

    for event in &tx.uniswap_v3_decreases {
        push_v3_liquidity_edge(
            &mut batch,
            &context,
            context_from_address(&context),
            event.pool_address,
            Some(event.log_index),
            "uniswap_v3_decrease_liquidity",
            event.liquidity,
            event.amount0,
            event.amount1,
        );
    }

    for event in &tx.uniswap_v4_initializes {
        if !pair_contains_tracked_token(event.currency0, event.currency1, &tracked_token_address) {
            continue;
        }
        let pool_node = v4_pool_node(event.event_id);
        push_pool_label(
            &mut batch,
            &context,
            pool_node.clone(),
            Some(event.log_index),
        );
        let mut edge = pool_edge(
            &context,
            token_node.clone(),
            pool_node,
            NetworkEdgeKind::PoolCreation,
            NetworkEvidenceSource::PoolState,
            Some(event.log_index),
            "uniswap_v4_initialize",
            "tracked token Uniswap V4 pool initialized",
        );
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V4_PROTOCOL.to_string());
        edge.attributes.insert(
            "pool_manager".to_string(),
            address_string(&event.pool_manager_address),
        );
        edge.attributes
            .insert("currency0".to_string(), address_string(&event.currency0));
        edge.attributes
            .insert("currency1".to_string(), address_string(&event.currency1));
        edge.attributes
            .insert("fee".to_string(), event.fee.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v4_swaps {
        let mut edge = pool_edge(
            &context,
            NetworkNodeId::address(address_string(&event.sender)),
            v4_pool_node(event.event_id),
            NetworkEdgeKind::PoolTrade,
            NetworkEvidenceSource::PoolTrade,
            Some(event.log_index),
            "uniswap_v4_swap",
            "Uniswap V4 swap involving pool",
        );
        edge.attributes
            .insert("amount0".to_string(), event.amount0.to_string());
        edge.attributes
            .insert("amount1".to_string(), event.amount1.to_string());
        edge.attributes
            .insert("tick".to_string(), event.tick.to_string());
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V4_PROTOCOL.to_string());
        batch.push_edge(edge);
    }

    for event in &tx.uniswap_v4_modifies {
        let mut edge = pool_edge(
            &context,
            NetworkNodeId::address(address_string(&event.sender)),
            v4_pool_node(event.event_id),
            NetworkEdgeKind::LiquidityEvent,
            NetworkEvidenceSource::PoolState,
            Some(event.log_index),
            "uniswap_v4_modify_liquidity",
            "Uniswap V4 liquidity modified",
        );
        edge.attributes.insert(
            "liquidity_delta".to_string(),
            event.liquidity_delta.to_string(),
        );
        edge.attributes
            .insert("protocol".to_string(), UNISWAP_V4_PROTOCOL.to_string());
        batch.push_edge(edge);
    }

    batch
}

fn push_pool_label(
    batch: &mut NetworkIngestBatch,
    context: &TransactionIngestContext,
    pool_node: NetworkNodeId,
    log_index: Option<u64>,
) {
    batch.push_label(NodeLabelIngest::observed(
        pool_node,
        NetworkLabelKind::Pool,
        NetworkLabelSource::PoolState,
        context.observation(log_index),
        NetworkEvidenceSource::PoolState,
        "pool observed in transaction",
    ));
}

fn pool_edge(
    context: &TransactionIngestContext,
    source: NetworkNodeId,
    target: NetworkNodeId,
    edge_kind: NetworkEdgeKind,
    evidence_source: NetworkEvidenceSource,
    log_index: Option<u64>,
    event_name: &str,
    description: &str,
) -> crate::network::model::NetworkEdge {
    let mut evidence = NetworkEvidence::new(evidence_source)
        .with_observation(context.observation(log_index))
        .with_description(description);
    evidence
        .attributes
        .insert("event".to_string(), event_name.to_string());
    edge_with_evidence(source, target, edge_kind, evidence)
}

#[allow(clippy::too_many_arguments)]
fn push_v3_liquidity_edge(
    batch: &mut NetworkIngestBatch,
    context: &TransactionIngestContext,
    owner: Address,
    pool_address: Address,
    log_index: Option<u64>,
    event_name: &str,
    liquidity: U256,
    amount0: U256,
    amount1: U256,
) {
    let mut edge = pool_edge(
        context,
        NetworkNodeId::address(address_string(&owner)),
        NetworkNodeId::pool_address(address_string(&pool_address)),
        NetworkEdgeKind::LiquidityEvent,
        NetworkEvidenceSource::PoolState,
        log_index,
        event_name,
        "Uniswap V3 liquidity event",
    );
    edge.attributes
        .insert("liquidity".to_string(), liquidity.to_string());
    edge.attributes
        .insert("amount0".to_string(), amount0.to_string());
    edge.attributes
        .insert("amount1".to_string(), amount1.to_string());
    edge.attributes
        .insert("protocol".to_string(), UNISWAP_V3_PROTOCOL.to_string());
    batch.push_edge(edge);
}

fn add_v2_amount_attrs(
    edge: &mut crate::network::model::NetworkEdge,
    amount0_in: U256,
    amount1_in: U256,
    amount0_out: U256,
    amount1_out: U256,
) {
    edge.attributes
        .insert("amount0_in".to_string(), amount0_in.to_string());
    edge.attributes
        .insert("amount1_in".to_string(), amount1_in.to_string());
    edge.attributes
        .insert("amount0_out".to_string(), amount0_out.to_string());
    edge.attributes
        .insert("amount1_out".to_string(), amount1_out.to_string());
}

fn pair_contains_tracked_token(
    token0: Address,
    token1: Address,
    tracked_token_address: &str,
) -> bool {
    address_string(&token0) == tracked_token_address
        || address_string(&token1) == tracked_token_address
}

fn v4_pool_node(pool_id: B256) -> NetworkNodeId {
    NetworkNodeId::pool(NetworkPoolId::pool_id(format!("{pool_id:#x}")))
}

fn context_from_address(context: &TransactionIngestContext) -> Address {
    context
        .from_address
        .parse()
        .unwrap_or_else(|_| Address::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, U256};
    use tx_processor::tx_processor::data_models::{
        UniswapV2PairCreatedEvent, UniswapV2SwapEvent, UniswapV3PoolCreatedEvent,
    };

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            3,
            address!("1111111111111111111111111111111111111111"),
            None,
            U256::ZERO,
            true,
            7,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn pool_ingest_extracts_v2_pair_creation_for_tracked_token() {
        let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let mut tx = tx();
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("9999999999999999999999999999999999999999"),
                token0: token,
                token1: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                factory_address: Address::ZERO,
                log_index: 4,
            });

        let batch = extract_pool_updates(&tx, address_string(&token));

        assert_eq!(batch.node_labels.len(), 1);
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, NetworkEdgeKind::PoolCreation);
        assert_eq!(
            batch.edges[0]
                .attributes
                .get("protocol")
                .map(String::as_str),
            Some(UNISWAP_V2_PROTOCOL)
        );
    }

    #[test]
    fn pool_ingest_extracts_v2_swap_edges() {
        let mut tx = tx();
        tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
            pair_address: address!("9999999999999999999999999999999999999999"),
            sender: address!("1111111111111111111111111111111111111111"),
            to: address!("2222222222222222222222222222222222222222"),
            amount0_in: U256::from(1_u64),
            amount1_in: U256::ZERO,
            amount0_out: U256::ZERO,
            amount1_out: U256::from(2_u64),
            log_index: 8,
        });

        let batch = extract_pool_updates(&tx, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, NetworkEdgeKind::PoolTrade);
        assert_eq!(
            batch.edges[0]
                .attributes
                .get("amount1_out")
                .map(String::as_str),
            Some("2")
        );
    }

    #[test]
    fn pool_ingest_extracts_v3_pool_creation_for_tracked_token() {
        let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let mut tx = tx();
        tx.uniswap_v3_pools.push(UniswapV3PoolCreatedEvent {
            token0: token,
            token1: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            fee: 3_000,
            tick_spacing: 60,
            pool: address!("9999999999999999999999999999999999999999"),
            log_index: 4,
        });

        let batch = extract_pool_updates(&tx, address_string(&token));

        assert_eq!(batch.node_labels.len(), 1);
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(
            batch.edges[0]
                .attributes
                .get("protocol")
                .map(String::as_str),
            Some(UNISWAP_V3_PROTOCOL)
        );
    }
}
