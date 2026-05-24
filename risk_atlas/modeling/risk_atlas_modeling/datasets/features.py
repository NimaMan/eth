"""As-of feature selection and preprocessing."""

from __future__ import annotations

import numpy as np
import pandas as pd


NUMERIC_FEATURES = [
    "active_observation_index",
    "tx_count",
    "token_transfer_count",
    "denom_transfer_count",
    "buy_volume_denom",
    "sell_volume_denom",
    "total_bribe_eth",
    "total_liquidity_denom",
    "denom_reserve",
    "token_reserve",
    "pooled_token_supply_ratio",
    "price_to_initial_ratio",
    "price_denom_per_token",
    "initial_price_denom_per_token",
    "buy_tax",
    "sell_tax",
    "lp_approved_pct_as_of",
    "lp_approval_count_in_block",
    "lp_total_supply",
    "lp_max_approval_amount_as_of",
    "lp_max_approval_pct_as_of",
    "lp_removable_pct_as_of",
    "lp_router_removable_pct_as_of",
    "creator_lp_balance_pct_as_of",
    "creator_lp_approved_pct_as_of",
    "creator_lp_removable_pct_as_of",
    "creator_lp_router_removable_pct_as_of",
    "last_lp_approval_amount_pct_of_total_supply",
    "first_lp_approval_to_as_of_chain_block_delta",
    "last_lp_approval_to_as_of_chain_block_delta",
    "pool_creation_to_first_lp_approval_chain_block_delta",
    "pool_creation_to_last_lp_approval_chain_block_delta",
    "trading_enabled_to_first_lp_approval_chain_block_delta",
    "trading_enabled_to_last_lp_approval_chain_block_delta",
    "pair_token_to_control_to_pool_reserve_ratio",
    "last_pair_balance_backdoor_signal_to_as_of_chain_block_delta",
    "token_transfer_to_total_supply_ratio",
    "token_transfer_to_pool_token_reserve_ratio",
]

BOOLEAN_FEATURES = [
    "can_buy",
    "can_sell",
    "effective_can_buy",
    "effective_can_sell",
    "price_to_initial_ratio_trustworthy",
    "liquidity_removed_as_of",
    "liquidity_removal_in_block",
    "direct_lp_removal_as_of",
    "direct_lp_removal_in_block",
    "lp_approval_seen_as_of",
    "lp_approval_owner_is_creator",
    "creator_lp_approved_gt_90_pct_as_of",
    "creator_lp_router_removable_gt_90_pct_as_of",
    "control_transfer_from_after_renounce_seen_as_of",
    "control_transfer_from_after_renounce_in_block",
    "control_transfer_from_holder_to_burn_seen_as_of",
    "control_transfer_from_holder_to_burn_in_block",
    "control_transfer_from_pair_seen_as_of",
    "control_transfer_from_pair_in_block",
    "control_transfer_from_without_transfer_log_seen_as_of",
    "control_transfer_from_without_transfer_log_in_block",
    "pair_token_to_control_seen_as_of",
    "pair_token_to_control_in_block",
    "pair_balance_backdoor_signal_seen_as_of",
    "pair_balance_backdoor_signal_in_block",
]


def feature_columns(frame: pd.DataFrame) -> list[str]:
    return [column for column in [*NUMERIC_FEATURES, *BOOLEAN_FEATURES] if column in frame.columns]


def feature_matrix(frame: pd.DataFrame) -> pd.DataFrame:
    columns = feature_columns(frame)
    features = frame[columns].copy()
    for column in BOOLEAN_FEATURES:
        if column in features.columns:
            features[column] = features[column].map({True: 1, False: 0}).fillna(0).astype(float)
    for column in features.columns:
        features[column] = pd.to_numeric(features[column], errors="coerce")
    features = features.replace([np.inf, -np.inf], np.nan)
    features = features.clip(lower=-1e12, upper=1e12)
    return features
