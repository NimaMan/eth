use std::collections::{BTreeMap, BTreeSet};

use crate::erc20::ERC20Token;

use super::report::{
    AuthoritySurfaceReport, BehaviorFlagKind, ContractAnalysisReport, ContractEvidence,
    ContractEvidenceSource, ContractFieldStatus, ContractSeverity, Erc20InterfaceQuality,
    Erc20InterfaceReport, PoolSurfaceReport, SupplySurfaceReport, TokenMetadataQualityReport,
    TransferSurfaceReport,
};

pub struct Erc20ContractAnalyzer;

impl Erc20ContractAnalyzer {
    pub fn analyze(token: &ERC20Token) -> ContractAnalysisReport {
        analyze_erc20_token(token)
    }
}

pub fn analyze_erc20_token(token: &ERC20Token) -> ContractAnalysisReport {
    let metadata = metadata_quality(token);
    let supply = supply_surface(token);
    let authority = authority_surface(token);
    let transfer = transfer_surface(token);
    let pools = pool_surface(token);

    let interface = Erc20InterfaceReport {
        quality: interface_quality(&metadata),
        metadata_complete: metadata_complete(&metadata),
        observed_transfers: transfer.transfer_tx_count > 0,
        observed_approvals: transfer.approval_count > 0,
    };

    let mut evidence = Vec::new();
    append_metadata_evidence(&metadata, &mut evidence);
    append_supply_evidence(token, &supply, &mut evidence);
    append_authority_evidence(&authority, &mut evidence);
    append_status_evidence(token, &pools, &mut evidence);
    append_pool_evidence(&pools, &mut evidence);

    let behavior_flags = evidence.iter().map(|item| item.kind).collect();

    ContractAnalysisReport {
        address: token.contract_address.clone(),
        block_number: token.latest_block_number.or(token.creation_block),
        timestamp: token.latest_block_timestamp.or(token.creation_timestamp),
        interface,
        metadata,
        supply,
        authority,
        transfer,
        pools,
        behavior_flags,
        evidence,
    }
}

fn metadata_quality(token: &ERC20Token) -> TokenMetadataQualityReport {
    TokenMetadataQualityReport {
        name: text_status(&token.name),
        symbol: text_status(&token.symbol),
        decimals: decimals_status(token.decimals),
        total_supply: total_supply_status(&token.total_supply),
    }
}

fn text_status(value: &str) -> ContractFieldStatus {
    if value.trim().is_empty() {
        ContractFieldStatus::Empty
    } else {
        ContractFieldStatus::Present
    }
}

fn decimals_status(value: u8) -> ContractFieldStatus {
    if value <= 36 {
        ContractFieldStatus::Present
    } else {
        ContractFieldStatus::Invalid
    }
}

fn total_supply_status(value: &str) -> ContractFieldStatus {
    let value = value.trim();
    if value.is_empty() {
        return ContractFieldStatus::Empty;
    }

    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return if !hex.is_empty() && hex.chars().all(|value| value.is_ascii_hexdigit()) {
            ContractFieldStatus::Present
        } else {
            ContractFieldStatus::Invalid
        };
    }

    if value.chars().all(|value| value.is_ascii_digit()) {
        ContractFieldStatus::Present
    } else {
        ContractFieldStatus::Invalid
    }
}

fn interface_quality(metadata: &TokenMetadataQualityReport) -> Erc20InterfaceQuality {
    if metadata_complete(metadata) {
        return Erc20InterfaceQuality::Complete;
    }

    if metadata.name == ContractFieldStatus::Invalid
        || metadata.symbol == ContractFieldStatus::Invalid
        || metadata.decimals == ContractFieldStatus::Invalid
        || metadata.total_supply == ContractFieldStatus::Invalid
    {
        return Erc20InterfaceQuality::NonStandard;
    }

    Erc20InterfaceQuality::MetadataIncomplete
}

fn metadata_complete(metadata: &TokenMetadataQualityReport) -> bool {
    metadata.name == ContractFieldStatus::Present
        && metadata.symbol == ContractFieldStatus::Present
        && metadata.decimals == ContractFieldStatus::Present
        && metadata.total_supply == ContractFieldStatus::Present
}

fn supply_surface(token: &ERC20Token) -> SupplySurfaceReport {
    let declared_total_supply_scaled = token.total_supply_scaled();
    let minted_from_transfers = token.total_supply_from_transfers();
    let minted_to_declared_ratio = declared_total_supply_scaled
        .filter(|supply| *supply > 0.0)
        .map(|supply| minted_from_transfers / supply);

    SupplySurfaceReport {
        declared_total_supply_raw: token.total_supply.clone(),
        declared_total_supply_scaled,
        minted_from_transfers,
        minted_to_declared_ratio,
        hidden_mint_detected: token.hidden_mint_detected()
            || minted_to_declared_ratio
                .map(|ratio| ratio > token.status_manager.hidden_mint_threshold)
                .unwrap_or(false),
    }
}

fn authority_surface(token: &ERC20Token) -> AuthoritySurfaceReport {
    let current_owner = token.current_owner();
    let ownership_renounced = token.ownership_renounced();
    AuthoritySurfaceReport {
        has_active_owner_control: current_owner.is_some() && !ownership_renounced,
        current_owner,
        ownership_renounced,
        control_address_count: token.authority_tracker.token_control_addresses.len(),
        owner_event_count: token.authority_tracker.owner_events.len(),
    }
}

fn transfer_surface(token: &ERC20Token) -> TransferSurfaceReport {
    TransferSurfaceReport {
        transfer_tx_count: token.transfer_tracker.erc20_transfers.len(),
        approval_count: token.transfer_tracker.approvals.len(),
        approved_spender_count: token.transfer_tracker.approved_addresses.len(),
        unique_address_count: token.unique_addresses().len(),
        bribe_total_eth: token.total_bribe_amount(),
    }
}

fn pool_surface(token: &ERC20Token) -> PoolSurfaceReport {
    let mut protocols = BTreeSet::new();
    let mut pool_count_by_protocol = BTreeMap::new();
    let mut trading_pool_count = 0;
    let mut cannot_sell_pool_count = 0;
    let mut liquidity_removal_pool_count = 0;

    for pool in token.all_pool_bases() {
        let protocol = pool.identity.protocol.clone();
        protocols.insert(protocol.clone());
        *pool_count_by_protocol.entry(protocol).or_insert(0) += 1;

        if pool.trading_enabled() {
            trading_pool_count += 1;
        }
        if pool.state.can_buy && !pool.state.can_sell {
            cannot_sell_pool_count += 1;
        }
        if pool.has_liquidity_removal() {
            liquidity_removal_pool_count += 1;
        }
    }

    PoolSurfaceReport {
        pool_count: token.pool_count(),
        protocols: protocols.into_iter().collect(),
        pool_count_by_protocol,
        trading_pool_count,
        cannot_sell_pool_count,
        liquidity_removal_pool_count,
    }
}

fn append_metadata_evidence(
    metadata: &TokenMetadataQualityReport,
    evidence: &mut Vec<ContractEvidence>,
) {
    if metadata_complete(metadata) {
        return;
    }

    let invalid = [
        metadata.name,
        metadata.symbol,
        metadata.decimals,
        metadata.total_supply,
    ]
    .contains(&ContractFieldStatus::Invalid);

    let kind = if invalid {
        BehaviorFlagKind::InvalidMetadata
    } else {
        BehaviorFlagKind::MetadataIncomplete
    };
    let severity = if invalid {
        ContractSeverity::Medium
    } else {
        ContractSeverity::Low
    };

    evidence.push(ContractEvidence::new(
        kind,
        severity,
        ContractEvidenceSource::Metadata,
        format!(
            "metadata quality: name={:?}, symbol={:?}, decimals={:?}, total_supply={:?}",
            metadata.name, metadata.symbol, metadata.decimals, metadata.total_supply
        ),
    ));
}

fn append_supply_evidence(
    token: &ERC20Token,
    supply: &SupplySurfaceReport,
    evidence: &mut Vec<ContractEvidence>,
) {
    if !supply.hidden_mint_detected {
        return;
    }

    let ratio = supply
        .minted_to_declared_ratio
        .map(|ratio| format!("{ratio:.6}"))
        .unwrap_or_else(|| "unknown".to_string());
    evidence.push(
        ContractEvidence::new(
            BehaviorFlagKind::HiddenMintEvidence,
            ContractSeverity::Critical,
            ContractEvidenceSource::Supply,
            format!(
                "transfer-derived minted supply exceeds declared supply ratio threshold; ratio={ratio}"
            ),
        )
        .with_block(token.status_manager.scam_block)
        .with_tx_hash(token.status_manager.scam_tx.clone()),
    );
}

fn append_authority_evidence(
    authority: &AuthoritySurfaceReport,
    evidence: &mut Vec<ContractEvidence>,
) {
    if !authority.has_active_owner_control {
        return;
    }

    evidence.push(ContractEvidence::new(
        BehaviorFlagKind::ActiveOwnerControl,
        ContractSeverity::Low,
        ContractEvidenceSource::Authority,
        "owner/control address is still active; treat as control-surface evidence",
    ));
}

fn append_status_evidence(
    token: &ERC20Token,
    pools: &PoolSurfaceReport,
    evidence: &mut Vec<ContractEvidence>,
) {
    if token.status_manager.trading_enabled && pools.trading_pool_count == 0 {
        evidence.push(
            ContractEvidence::new(
                BehaviorFlagKind::RawTradingEventWithoutPoolTrading,
                ContractSeverity::Medium,
                ContractEvidenceSource::Status,
                "raw token trading event exists but no pool-derived trading state exists",
            )
            .with_block(token.status_manager.trading_enabled_block)
            .with_tx_hash(token.status_manager.trading_enabled_tx.clone()),
        );
    }
}

fn append_pool_evidence(pools: &PoolSurfaceReport, evidence: &mut Vec<ContractEvidence>) {
    if pools.cannot_sell_pool_count > 0 {
        evidence.push(ContractEvidence::new(
            BehaviorFlagKind::CannotSellPool,
            ContractSeverity::High,
            ContractEvidenceSource::Pools,
            format!(
                "{} pool(s) can buy but cannot sell",
                pools.cannot_sell_pool_count
            ),
        ));
    }

    if pools.liquidity_removal_pool_count > 0 {
        evidence.push(ContractEvidence::new(
            BehaviorFlagKind::PoolLiquidityRemovalEvidence,
            ContractSeverity::High,
            ContractEvidenceSource::Pools,
            format!(
                "{} pool(s) show liquidity-removal evidence",
                pools.liquidity_removal_pool_count
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erc20::ERC20TokenMetadata;

    fn token() -> ERC20Token {
        ERC20Token::new(ERC20TokenMetadata::new(
            "0x0000000000000000000000000000000000000001",
            "Token",
            "TKN",
            18,
            "100000000000000000000",
        ))
    }

    #[test]
    fn complete_metadata_reports_complete_interface() {
        let report = analyze_erc20_token(&token());

        assert_eq!(report.interface.quality, Erc20InterfaceQuality::Complete);
        assert!(report.interface.metadata_complete);
        assert!(report.evidence.is_empty());
    }

    #[test]
    fn empty_metadata_becomes_evidence_not_a_hard_failure() {
        let mut token = token();
        token.symbol.clear();

        let report = analyze_erc20_token(&token);

        assert_eq!(
            report.interface.quality,
            Erc20InterfaceQuality::MetadataIncomplete
        );
        assert!(report
            .behavior_flags
            .contains(&BehaviorFlagKind::MetadataIncomplete));
    }

    #[test]
    fn hidden_mint_is_reported_from_transfer_derived_supply() {
        let mut token = token();
        token.transfer_tracker.total_supply_from_transfers = 102.0;

        let report = analyze_erc20_token(&token);

        assert!(report.supply.hidden_mint_detected);
        assert!(report
            .behavior_flags
            .contains(&BehaviorFlagKind::HiddenMintEvidence));
    }

    #[test]
    fn raw_trading_event_without_pool_trading_is_separate_evidence() {
        let mut token = token();
        token.status_manager.trading_enabled = true;
        token.status_manager.trading_enabled_block = Some(100);

        let report = analyze_erc20_token(&token);

        assert!(!token.trading_enabled());
        assert!(report
            .behavior_flags
            .contains(&BehaviorFlagKind::RawTradingEventWithoutPoolTrading));
    }
}
