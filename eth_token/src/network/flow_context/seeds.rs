//! Seed address selection from the raw token network.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::network::{
    config::TokenNetworkGraphConfig,
    graph::RawTokenNetworkGraph,
    model::{NetworkLabelKind, NetworkNodeId},
};

use super::{config::FlowContextConfig, model::FlowContextNode};

/// Why an address was selected as a second-order fund-flow seed.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowContextSeedKind {
    ControlActor,
    ProfitableTrader,
    Holder,
    ActiveTrader,
    FeeSource,
    LpActor,
    AddressActivity,
}

/// One selected seed address.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextSeed {
    pub address: String,
    pub kind: FlowContextSeedKind,
    pub score: f64,
    pub labels: Vec<String>,
    pub first_seen_block: Option<u64>,
    pub last_seen_block: Option<u64>,
}

impl FlowContextSeed {
    pub fn as_node(&self) -> FlowContextNode {
        FlowContextNode::seed(&self.address, self.score, self.labels.clone())
    }
}

/// Select bounded, high-signal addresses from the token graph.
pub fn select_flow_context_seeds(
    graph: &RawTokenNetworkGraph,
    config: &FlowContextConfig,
) -> Vec<FlowContextSeed> {
    let mut seeds = graph
        .address_activity
        .values()
        .map(|activity| {
            let labels = graph
                .node(&activity.node_id)
                .map(node_label_keys)
                .unwrap_or_default();
            let kind = seed_kind(&labels, activity);
            let pnl = activity.pnl_proxy(None);
            let score = pnl.total_profit.abs()
                + activity.token_balance().abs()
                + activity.denom_balance().abs()
                + activity.num_tx_proxy() as f64
                + label_bonus(&labels);

            FlowContextSeed {
                address: activity.address.clone(),
                kind,
                score,
                labels,
                first_seen_block: activity
                    .observed
                    .first_seen
                    .as_ref()
                    .map(|observation| observation.block_number),
                last_seen_block: activity
                    .observed
                    .last_seen
                    .as_ref()
                    .map(|observation| observation.block_number),
            }
        })
        .collect::<Vec<_>>();

    for node in graph.nodes.values() {
        let NetworkNodeId::Address(address) = &node.id else {
            continue;
        };
        if graph.address_activity.contains_key(address) {
            continue;
        }
        let labels = node_label_keys(node);
        if !has_control_or_lp_label(&labels) {
            continue;
        }
        seeds.push(FlowContextSeed {
            address: address.clone(),
            kind: if has_control_label(&labels) {
                FlowContextSeedKind::ControlActor
            } else {
                FlowContextSeedKind::LpActor
            },
            score: 10_000.0 + label_bonus(&labels),
            labels,
            first_seen_block: node
                .observed
                .first_seen
                .as_ref()
                .map(|observation| observation.block_number),
            last_seen_block: node
                .observed
                .last_seen
                .as_ref()
                .map(|observation| observation.block_number),
        });
    }

    seeds.sort_by(|left, right| {
        compare_f64_desc(left.score, right.score).then_with(|| left.address.cmp(&right.address))
    });
    seeds.dedup_by(|left, right| left.address == right.address);
    seeds.truncate(config.max_seed_addresses);
    seeds
}

fn node_label_keys(node: &crate::network::model::NetworkNode) -> Vec<String> {
    node.labels
        .iter()
        .map(|label| label.kind.stable_key())
        .collect()
}

fn seed_kind(
    labels: &[String],
    activity: &crate::network::activity::AddressActivity,
) -> FlowContextSeedKind {
    if has_control_label(labels) {
        FlowContextSeedKind::ControlActor
    } else if labels
        .iter()
        .any(|label| label == "lp_holder" || label == "lp_approver")
    {
        FlowContextSeedKind::LpActor
    } else if activity.is_fee_source {
        FlowContextSeedKind::FeeSource
    } else if activity.pnl_proxy(None).total_profit > 0.0 {
        FlowContextSeedKind::ProfitableTrader
    } else if activity.token_balance() > 0.0 {
        FlowContextSeedKind::Holder
    } else if activity.num_tx_proxy() > 0 {
        FlowContextSeedKind::ActiveTrader
    } else {
        FlowContextSeedKind::AddressActivity
    }
}

fn has_control_or_lp_label(labels: &[String]) -> bool {
    has_control_label(labels)
        || labels.iter().any(|label| {
            label == "lp_holder" || label == "lp_approver" || label == "liquidity_actor"
        })
}

fn has_control_label(labels: &[String]) -> bool {
    [
        NetworkLabelKind::Creator.stable_key(),
        NetworkLabelKind::Owner.stable_key(),
        NetworkLabelKind::PendingOwner.stable_key(),
        NetworkLabelKind::Admin.stable_key(),
        NetworkLabelKind::ProxyAdmin.stable_key(),
        NetworkLabelKind::TaxWallet.stable_key(),
        NetworkLabelKind::ControlActor.stable_key(),
    ]
    .iter()
    .any(|kind| labels.iter().any(|label| label == kind))
}

fn label_bonus(labels: &[String]) -> f64 {
    if labels.is_empty() {
        return 0.0;
    }
    labels
        .iter()
        .map(|label| match label.as_str() {
            "creator" | "owner" | "admin" | "proxy_admin" | "tax_wallet" => 25_000.0,
            "lp_holder" | "lp_approver" | "liquidity_actor" => 10_000.0,
            "wallet" => 10.0,
            _ => 1.0,
        })
        .sum()
}

fn compare_f64_desc(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

#[allow(dead_code)]
fn _keep_config_import_used(_: &TokenNetworkGraphConfig) {}
