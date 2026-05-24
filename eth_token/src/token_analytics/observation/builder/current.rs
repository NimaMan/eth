use std::collections::BTreeMap;

use crate::erc20::ERC20Token;
use crate::pools::{BasePool, TaxBucket};
use crate::token_activity::TokenBlockActivity;
use crate::token_analytics::{
    ActiveObservationReason, FeatureEvidenceBlocks, ObservationBlockActivity,
    ObservationBlockActivitySource, ObservationPoolTradingState, ObservationTransactionSummary,
    ObservationTransactionType, ObservedSellTransferFlow, PoolActivityFeatures, PoolMarketFeatures,
    TokenAuthorityFeatures, TokenNetworkFeatures, TokenPoolCurrentObservation,
    TokenPoolObservationContext, TokenPoolObservationFeatures, TokenPoolObservationKey,
    TokenStaticFeatures,
};
use crate::tracking::{TokenBlockUpdateReport, TokenRegistry};

use super::actions::block_actions;
use super::activity::apply_cumulative_activity_features;
use super::flags::event_flags;
use super::liquidity::liquidity_features;
use super::lp_control::lp_control_features;
use super::reasons::{append_activity_reasons, append_event_reasons};
use super::token_control::token_control_features;
use super::transfers::{block_transfer_summary, observed_sell_flow, token_pool_movement};
use super::utils::{
    activity_has_signal_for_denom, display_tax, economic_sellable, normalize_address,
    observation_pool_key, pool_token_reserve_ratio_denominator, tax_bucket_key,
};

pub fn collect_current_observations(
    registry: &TokenRegistry,
    report: &TokenBlockUpdateReport,
    active_observation_counts_by_pool: &mut BTreeMap<String, u64>,
) -> Vec<TokenPoolCurrentObservation> {
    let mut touched = BTreeMap::<(String, String), Vec<ActiveObservationReason>>::new();

    for update in &report.token_updates {
        for pool_address in update
            .discovered_known_v2_pools
            .iter()
            .chain(update.discovered_uniswap_v3_pools.iter())
            .chain(update.discovered_uniswap_v4_pools.iter())
        {
            push_reason(
                &mut touched,
                &update.token_address,
                pool_address,
                ActiveObservationReason::PoolLiquidityChange,
            );
        }

        for pool_address in update
            .updated_known_v2_pools
            .iter()
            .chain(update.updated_uniswap_v3_pools.iter())
            .chain(update.updated_uniswap_v4_pools.iter())
        {
            push_reason(
                &mut touched,
                &update.token_address,
                pool_address,
                ActiveObservationReason::PoolSync,
            );
        }

        for pool_address in update
            .simulated_known_v2_pools
            .iter()
            .chain(update.simulated_uniswap_v3_pools.iter())
            .chain(update.simulated_uniswap_v4_pools.iter())
        {
            push_reason(
                &mut touched,
                &update.token_address,
                pool_address,
                ActiveObservationReason::TradingStatusChange,
            );
        }

        if update.token_state_updated {
            let Some(token) = registry.token(&update.token_address) else {
                continue;
            };
            for pool in token.all_pool_bases() {
                let reason = token_state_activity_reason(token, pool, report.block_number);
                push_reason(
                    &mut touched,
                    &update.token_address,
                    &pool.identity.pool_address,
                    reason,
                );
            }
        }
    }

    let mut observations = Vec::with_capacity(touched.len());
    for ((token_address, pool_address), seed_reasons) in touched {
        let Some(token) = registry.token(&token_address) else {
            continue;
        };
        let Some(pool) = token.pool_base(&pool_address) else {
            continue;
        };
        let pool_key = observation_pool_key(&token_address, &pool_address);
        let active_index = {
            let count = active_observation_counts_by_pool
                .entry(pool_key)
                .or_default();
            *count += 1;
            *count
        };
        let activity = token.activity.blocks.get(&report.block_number);
        let observation = build_current_observation(
            token,
            pool,
            active_index,
            report.block_number,
            Some(report.block_timestamp),
            seed_reasons,
            activity,
        );
        observations.push(observation);
    }

    observations
}

fn token_state_activity_reason(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
) -> ActiveObservationReason {
    if block_has_control_address_activity(token, block_number) {
        return ActiveObservationReason::ControlAddressActivity;
    }
    if token
        .activity
        .blocks
        .get(&block_number)
        .is_some_and(|activity| {
            activity_has_signal_for_denom(activity, &pool.identity.denom_address)
        })
    {
        return ActiveObservationReason::NetworkActivity;
    }
    ActiveObservationReason::Manual
}

fn block_has_control_address_activity(token: &ERC20Token, block_number: u64) -> bool {
    let control_addresses = token
        .token_control_addresses
        .iter()
        .map(|address| normalize_address(address))
        .collect::<Vec<_>>();
    token
        .activity
        .transactions_in_block(block_number)
        .into_iter()
        .filter_map(|transaction| transaction.maker.as_deref())
        .any(|maker| {
            let maker = normalize_address(maker);
            control_addresses.iter().any(|control| control == &maker)
        })
}

fn push_reason(
    touched: &mut BTreeMap<(String, String), Vec<ActiveObservationReason>>,
    token_address: &str,
    pool_address: &str,
    reason: ActiveObservationReason,
) {
    let reasons = touched
        .entry((
            normalize_address(token_address),
            normalize_address(pool_address),
        ))
        .or_default();
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

pub fn build_current_observation(
    token: &ERC20Token,
    pool: &BasePool,
    active_index: u64,
    block_number: u64,
    timestamp: Option<u64>,
    seed_reasons: Vec<ActiveObservationReason>,
    block_activity: Option<&TokenBlockActivity>,
) -> TokenPoolCurrentObservation {
    let activity = block_activity
        .map(ObservationBlockActivity::from)
        .unwrap_or_else(|| ObservationBlockActivity {
            source: ObservationBlockActivitySource::TokenActivityTracker,
            metrics_complete: true,
            ..Default::default()
        });
    let event_flags = event_flags(token, pool, block_number);
    let mut reasons = seed_reasons;
    append_activity_reasons(&activity, &pool.identity.denom_address, &mut reasons);
    append_event_reasons(&event_flags, &mut reasons);
    if reasons.is_empty() {
        reasons.push(ActiveObservationReason::Manual);
    }

    let mut activity_features = PoolActivityFeatures::from(&activity);
    let denom = pool.identity.denom_address.as_str();
    activity_features.set_block_volume(
        activity.buy_volume_for_denom(denom),
        activity.sell_volume_for_denom(denom),
    );
    let pool_token_reserve_denominator =
        pool_token_reserve_ratio_denominator(token, pool, block_number);
    activity_features.set_token_transfer_volume_context(
        activity.token_transfer_volume,
        token.total_supply_scaled(),
        pool_token_reserve_denominator,
    );
    let transfer_summary = block_transfer_summary(token, block_number);
    let token_pool_movement = token_pool_movement(token, pool, block_number);
    let token_control = token_control_features(token, pool, block_number, &token_pool_movement);
    let sell_flow = observed_sell_flow(token, pool, block_number);
    activity_features.set_observed_sell_transfer_flow(ObservedSellTransferFlow {
        observed_sell_tx_count: sell_flow.sell_tx_count,
        seller_token_out: sell_flow.seller_token_out,
        seller_token_to_pool: sell_flow.seller_token_to_pool,
        seller_token_to_token_contract: sell_flow.seller_token_to_token_contract,
        seller_token_to_other: sell_flow.seller_token_to_other,
        token_contract_to_pool: sell_flow.token_contract_to_pool,
        pool_token_reserve: pool_token_reserve_denominator,
    });
    apply_cumulative_activity_features(
        &mut activity_features,
        token,
        pool,
        active_index,
        block_number,
    );

    let trading = ObservationPoolTradingState {
        lifecycle: Some(pool.state.lifecycle),
        can_buy: pool.state.can_buy,
        can_sell: pool.state.can_sell,
        effective_can_buy: pool.effective_can_buy(),
        effective_can_sell: pool.effective_can_sell(),
        economic_sellable: economic_sellable(pool.state.can_sell, pool.sell_tax),
        buy_tax: pool.buy_tax,
        sell_tax: pool.sell_tax,
        tax_check_block: pool.tax_check_block,
        tax_check_tx: pool.tax_check_tx.clone(),
        last_trading_failure_class: pool.last_trading_failure_class.clone(),
        liquidity_removed_as_of: pool.has_liquidity_removal(),
        liquidity_removal_block_as_of: pool.scam_block,
    };

    let mut token_features =
        TokenStaticFeatures::from_blocks(token.creation_block, pool.creation_block, block_number);
    token_features.decimals = Some(token.decimals);
    token_features.total_supply_scaled = token.total_supply_scaled();
    token_features.total_supply_from_transfers = Some(token.total_supply_from_transfers());

    let mut authority_features = TokenAuthorityFeatures::with_owner_context(
        token.creator_address.clone(),
        token.current_owner(),
        token.ownership_renounced(),
    );
    authority_features.renouncement_block = token.authority_tracker.renouncement_block;
    authority_features.control_address_count = Some(token.token_control_addresses.len() as u32);
    authority_features.control_address_tx_count_in_block =
        Some(pool.latest_block_control_address_txs.len() as u32);

    let scam_mechanism = pool.inferred_scam_mechanism();
    let mut market_features =
        PoolMarketFeatures::with_ages(pool.creation_block, pool.can_buy_block, block_number);
    market_features.lifecycle = Some(pool.state.lifecycle);
    market_features.can_buy = pool.state.can_buy;
    market_features.can_sell = pool.state.can_sell;
    market_features.effective_can_buy = pool.effective_can_buy();
    market_features.effective_can_sell = pool.effective_can_sell();
    market_features.economic_sellable = economic_sellable(pool.state.can_sell, pool.sell_tax);
    market_features.buy_tax = pool.buy_tax;
    market_features.sell_tax = pool.sell_tax;
    market_features.tax_bucket = Some(tax_bucket_key(TaxBucket::combined(
        display_tax(pool.buy_tax),
        display_tax(pool.sell_tax),
    )));
    market_features.last_trading_failure_class = pool.last_trading_failure_class.clone();
    market_features.liquidity_removed_as_of = pool.has_liquidity_removal();
    market_features.liquidity_removal_block_as_of = pool.scam_block;
    market_features.scam_mechanism_as_of = scam_mechanism
        .as_ref()
        .filter(|_| pool.has_liquidity_removal())
        .map(|mechanism| mechanism.mechanism.clone());
    market_features.scam_label_as_of = pool
        .scam_label
        .clone()
        .or_else(|| {
            scam_mechanism
                .as_ref()
                .map(|mechanism| mechanism.label.clone())
        })
        .filter(|_| pool.has_liquidity_removal());

    let liquidity_features = liquidity_features(token, pool);
    let lp_control = lp_control_features(token, &pool.identity.pool_address, block_number)
        .with_last_approval_offsets(pool.creation_block, pool.can_buy_block);

    let evidence_blocks = FeatureEvidenceBlocks {
        token_static_latest_block: token.creation_block,
        authority_latest_block: token
            .authority_tracker
            .owner_events
            .iter()
            .map(|event| event.block_number)
            .max(),
        market_latest_block: [
            pool.creation_block,
            pool.can_buy_block,
            pool.tax_check_block,
            pool.scam_block,
            Some(block_number),
        ]
        .into_iter()
        .flatten()
        .max(),
        liquidity_latest_block: pool
            .reserve_tracker
            .latest_snapshot
            .as_ref()
            .map(|snapshot| snapshot.block_number),
        lp_control_latest_block: lp_control.last_lp_approval_block,
        token_control_latest_block: token_control
            .last_pair_balance_backdoor_signal_block
            .or(token_control.last_control_transfer_from_block),
        activity_latest_block: Some(block_number),
        network_latest_block: None,
    };

    let transactions = token
        .activity
        .transactions_in_block(block_number)
        .into_iter()
        .map(|transaction| {
            let mut summary = ObservationTransactionSummary::from(transaction);
            if transaction
                .maker
                .as_deref()
                .is_some_and(|maker| token_control_addresses_contain(token, maker))
            {
                summary.add_type(ObservationTransactionType::ControlAddressActivity);
                summary.refresh_classification();
            }
            summary
        })
        .collect();
    let block_actions = block_actions(
        token,
        pool,
        block_number,
        &activity,
        &event_flags,
        &sell_flow,
    );

    TokenPoolCurrentObservation {
        key: TokenPoolObservationKey::new(
            &pool.identity.token_address,
            &pool.identity.pool_address,
            &pool.identity.denom_address,
            &pool.identity.protocol,
        )
        .with_chain("ethereum"),
        context: TokenPoolObservationContext::new(active_index, block_number, timestamp)
            .with_reasons(reasons),
        activity,
        event_flags,
        trading,
        features: TokenPoolObservationFeatures {
            token: token_features,
            authority: authority_features,
            market: market_features,
            liquidity: liquidity_features,
            lp_control,
            token_control,
            activity: activity_features,
            network: TokenNetworkFeatures {
                unique_address_count: Some(token.unique_addresses().len() as u32),
                feature_scope: Some("token_activity_tracker".to_string()),
                ..Default::default()
            },
            evidence_blocks,
        },
        transactions,
        transfer_summary: Some(transfer_summary),
        token_pool_movement: Some(token_pool_movement),
        sell_flow: Some(sell_flow),
        block_actions,
    }
}

fn token_control_addresses_contain(token: &ERC20Token, address: &str) -> bool {
    let address = normalize_address(address);
    token
        .token_control_addresses
        .iter()
        .map(|control| normalize_address(control))
        .any(|control| control == address)
}

#[cfg(test)]
mod tests {
    use crate::erc20::ERC20TokenMetadata;
    use crate::pools::BasePoolConfig;
    use crate::tracking::{TokenBlockUpdateReport, TokenRegistry, TokenStateUpdateReport};

    use super::*;

    const TOKEN: &str = "0x1111111111111111111111111111111111111111";
    const POOL: &str = "0x2222222222222222222222222222222222222222";
    const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    const CONTROL: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn token_state_update_creates_observation_without_pool_event() {
        let mut registry = TokenRegistry::new();
        registry.add_token(ERC20TokenMetadata::new(
            TOKEN,
            "Token",
            "TKN",
            18,
            "1000000000000000000",
        ));
        let token = registry.token_mut(TOKEN).expect("token exists");
        token.create_uniswap_v2_pool(POOL, WETH, BasePoolConfig::new(18), Vec::<&str>::new());
        token.token_control_addresses.insert(CONTROL.to_string());
        token
            .activity
            .record_transaction("0xaaaaaaaa", Some(CONTROL), 42, Some(1_700));

        let report = TokenBlockUpdateReport {
            block_number: 42,
            block_hash: "0xblock".to_string(),
            block_timestamp: 1_700,
            transaction_count: 1,
            processed_transaction_count: 1,
            failed_transaction_count: 0,
            pool_simulation_failure_count: 0,
            already_processed: false,
            created_token_addresses: Vec::new(),
            updated_token_addresses: vec![TOKEN.to_string()],
            token_updates: vec![TokenStateUpdateReport {
                token_address: TOKEN.to_string(),
                token_state_updated: true,
                ..Default::default()
            }],
            transaction_errors: Vec::new(),
        };

        let mut active_counts = BTreeMap::new();
        let observations = collect_current_observations(&registry, &report, &mut active_counts);

        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(observation.context.block_number, 42);
        assert!(observation
            .context
            .active_reasons
            .contains(&ActiveObservationReason::ControlAddressActivity));
        assert!(observation
            .block_actions
            .iter()
            .any(|action| action.key == "token_control_call"));
        assert!(observation
            .transactions
            .iter()
            .any(|transaction| transaction
                .tx_types
                .contains(&ObservationTransactionType::ControlAddressActivity)));
    }
}
