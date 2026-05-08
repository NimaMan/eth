use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use eth_token::erc20::ERC20Token;
use eth_token::network::{
    activity::AddressActivitySummary,
    graph::RawTokenNetworkGraph,
    model::{NetworkEdgeKind, NetworkNode, NetworkNodeId},
};
use serde::Serialize;

const MAX_PNL_ROWS: usize = 100;
const MAX_GRAPH_NODES: usize = 80;
const MAX_GRAPH_EDGES: usize = 160;

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkView {
    pub node_count: usize,
    pub edge_count: usize,
    pub address_count: usize,
    pub applied_batches: u64,
    pub pnl_rows: Vec<AddressActivitySummary>,
    pub pnl_omitted_count: usize,
    pub graph: TokenNetworkGraphView,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TokenNetworkGraphView {
    pub nodes: Vec<TokenNetworkNodeView>,
    pub edges: Vec<TokenNetworkEdgeView>,
    pub omitted_node_count: usize,
    pub omitted_edge_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkNodeView {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub address: Option<String>,
    pub labels: Vec<String>,
    pub activity_count: Option<u64>,
    pub token_balance: Option<f64>,
    pub denom_balance: Option<f64>,
    pub total_profit: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkEdgeView {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub weight: u64,
    pub amount: Option<f64>,
    pub weak: bool,
}

impl TokenNetworkView {
    pub fn from_graph(token: &ERC20Token, graph: Option<&RawTokenNetworkGraph>) -> Self {
        let Some(graph) = graph else {
            return Self {
                node_count: 0,
                edge_count: 0,
                address_count: 0,
                applied_batches: 0,
                pnl_rows: Vec::new(),
                pnl_omitted_count: 0,
                graph: TokenNetworkGraphView::default(),
            };
        };

        let latest_price = latest_token_price(token);
        let total_supply = token.total_supply_scaled();
        let mut pnl_rows = graph
            .address_activity
            .values()
            .map(|activity| activity.summary(latest_price, total_supply))
            .collect::<Vec<_>>();
        pnl_rows.sort_by(compare_pnl_rows);
        let pnl_omitted_count = pnl_rows.len().saturating_sub(MAX_PNL_ROWS);
        pnl_rows.truncate(MAX_PNL_ROWS);

        let summary_by_node = graph
            .address_activity
            .values()
            .map(|activity| {
                let summary = activity.summary(latest_price, total_supply);
                (summary.node_id.stable_key(), summary)
            })
            .collect::<BTreeMap<_, _>>();

        let graph_view = TokenNetworkGraphView::from_graph(graph, &summary_by_node);

        Self {
            node_count: graph.nodes.len(),
            edge_count: graph.edges.len(),
            address_count: graph.address_activity.len(),
            applied_batches: graph.applied_batches,
            pnl_rows,
            pnl_omitted_count,
            graph: graph_view,
        }
    }
}

impl TokenNetworkGraphView {
    fn from_graph(
        graph: &RawTokenNetworkGraph,
        summary_by_node: &BTreeMap<String, AddressActivitySummary>,
    ) -> Self {
        let mut scored_nodes = graph
            .nodes
            .values()
            .map(|node| {
                let key = node.id.stable_key();
                let score = node_score(node, summary_by_node.get(&key));
                (
                    score,
                    TokenNetworkNodeView::from_node(node, summary_by_node.get(&key)),
                )
            })
            .collect::<Vec<_>>();
        scored_nodes.sort_by(|left, right| {
            compare_f64_desc(left.0, right.0).then_with(|| left.1.id.cmp(&right.1.id))
        });
        scored_nodes.truncate(MAX_GRAPH_NODES);

        let nodes = scored_nodes
            .into_iter()
            .map(|(_, node)| node)
            .collect::<Vec<_>>();
        let selected_node_ids = nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();

        let mut edges = graph
            .edges
            .values()
            .filter(|edge| {
                selected_node_ids.contains(&edge.source().stable_key())
                    && selected_node_ids.contains(&edge.target().stable_key())
            })
            .map(TokenNetworkEdgeView::from_edge)
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            left.weak
                .cmp(&right.weak)
                .then_with(|| right.weight.cmp(&left.weight))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.target.cmp(&right.target))
        });
        let visible_edge_candidates = edges.len();
        edges.truncate(MAX_GRAPH_EDGES);

        Self {
            omitted_node_count: graph.nodes.len().saturating_sub(nodes.len()),
            omitted_edge_count: graph.edges.len().saturating_sub(visible_edge_candidates)
                + visible_edge_candidates.saturating_sub(edges.len()),
            nodes,
            edges,
        }
    }
}

impl TokenNetworkNodeView {
    fn from_node(node: &NetworkNode, summary: Option<&AddressActivitySummary>) -> Self {
        Self {
            id: node.id.stable_key(),
            kind: node.kind.stable_key(),
            label: node_label(&node.id),
            address: node_address(&node.id),
            labels: node
                .labels
                .iter()
                .map(|label| label.kind.stable_key())
                .collect(),
            activity_count: summary.map(|summary| summary.num_tx),
            token_balance: summary.map(|summary| summary.token_balance),
            denom_balance: summary.map(|summary| summary.denom_balance),
            total_profit: summary.map(|summary| summary.pnl.total_profit),
        }
    }
}

impl TokenNetworkEdgeView {
    fn from_edge(edge: &eth_token::network::model::NetworkEdge) -> Self {
        Self {
            source: edge.source().stable_key(),
            target: edge.target().stable_key(),
            kind: edge.kind.stable_key(),
            weight: edge.evidence.observed.count.max(1),
            amount: edge.amount.as_ref().and_then(|amount| amount.scaled_total),
            weak: is_weak_edge(&edge.kind),
        }
    }
}

fn latest_token_price(token: &ERC20Token) -> Option<f64> {
    token
        .all_pool_bases()
        .into_iter()
        .filter_map(|pool| {
            let price = pool.price();
            let liquidity = pool.state.total_liquidity;
            if price.is_finite() && price > 0.0 && liquidity.is_finite() && liquidity >= 0.0 {
                Some((liquidity, price))
            } else {
                None
            }
        })
        .max_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(Ordering::Equal))
        .map(|(_, price)| price)
}

fn compare_pnl_rows(left: &AddressActivitySummary, right: &AddressActivitySummary) -> Ordering {
    compare_f64_desc(left.pnl.total_profit, right.pnl.total_profit)
        .then_with(|| compare_f64_desc(left.denom_balance, right.denom_balance))
        .then_with(|| left.address.cmp(&right.address))
}

fn compare_f64_desc(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

fn node_score(node: &NetworkNode, summary: Option<&AddressActivitySummary>) -> f64 {
    match &node.id {
        NetworkNodeId::Token(_) => 1_000_000_000_000.0,
        NetworkNodeId::Pool(_) => 100_000_000_000.0 + node.observed.count as f64,
        NetworkNodeId::Address(_) => summary
            .map(|summary| {
                summary.pnl.total_profit.abs()
                    + summary.token_balance.abs()
                    + summary.denom_balance.abs()
                    + summary.num_tx as f64
            })
            .unwrap_or(node.observed.count as f64),
        _ => node.observed.count as f64,
    }
}

fn node_label(node_id: &NetworkNodeId) -> String {
    match node_id {
        NetworkNodeId::Token(address) => format!("token {}", short_id(address)),
        NetworkNodeId::Address(address) => short_id(address),
        NetworkNodeId::Pool(pool_id) => short_id(&pool_id.stable_key()),
        NetworkNodeId::TimeWindow(window) => format!("window {}", window.start_block),
        NetworkNodeId::Synthetic(id) => id.clone(),
    }
}

fn node_address(node_id: &NetworkNodeId) -> Option<String> {
    match node_id {
        NetworkNodeId::Token(address) | NetworkNodeId::Address(address) => Some(address.clone()),
        NetworkNodeId::Pool(eth_token::network::model::NetworkPoolId::Address(address)) => {
            Some(address.clone())
        }
        _ => None,
    }
}

fn short_id(value: &str) -> String {
    if value.len() <= 14 {
        return value.to_string();
    }
    format!(
        "{}...{}",
        &value[..8],
        &value[value.len().saturating_sub(6)..]
    )
}

fn is_weak_edge(kind: &NetworkEdgeKind) -> bool {
    matches!(
        kind,
        NetworkEdgeKind::FeeSourceTouches
            | NetworkEdgeKind::SharedIntermediary
            | NetworkEdgeKind::TemporalCoactivity
    )
}

#[cfg(test)]
mod tests {
    use eth_token::erc20::ERC20TokenMetadata;
    use eth_token::network::activity::{
        AddressActivity, AddressMovement, MovementAssetKind, MovementDirection,
    };
    use eth_token::network::model::{NetworkObservation, TokenNetworkId};

    use super::*;

    fn token() -> ERC20Token {
        ERC20Token::new(ERC20TokenMetadata::new(
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "Test",
            "TST",
            18,
            "1000000000000000000000",
        ))
    }

    fn activity(address: &str, denom_in: f64) -> AddressActivity {
        let mut activity = AddressActivity::new(address);
        activity.record_movement(AddressMovement::new(
            MovementDirection::In,
            MovementAssetKind::Native,
            denom_in,
            NetworkObservation::new(1),
        ));
        activity
    }

    #[test]
    fn network_view_sorts_and_caps_pnl_rows() {
        let token = token();
        let mut graph = RawTokenNetworkGraph::new(TokenNetworkId::new(
            None,
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ));
        for index in 0..105 {
            let address = format!("0x{index:040x}");
            graph
                .address_activity
                .insert(address.clone(), activity(&address, index as f64));
        }

        let view = TokenNetworkView::from_graph(&token, Some(&graph));

        assert_eq!(view.pnl_rows.len(), 100);
        assert_eq!(view.pnl_omitted_count, 5);
        assert_eq!(view.pnl_rows[0].pnl.total_profit, 104.0);
    }

    #[test]
    fn empty_network_view_has_stable_empty_payload() {
        let token = token();
        let view = TokenNetworkView::from_graph(&token, None);

        assert_eq!(view.node_count, 0);
        assert!(view.pnl_rows.is_empty());
        assert!(view.graph.nodes.is_empty());
    }
}
