use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use alloy_primitives::{Address, B256, U256};
use eyre::{eyre, Result, WrapErr};
use tx_fund_flow_fundflownetwork::extract_fund_flows_from_processed_tx;
use tx_processor::{ProcessedTransaction, ProcessedTxProvider};

use super::model::{
    EthMovementEvidence, ForwarderTrace, ScammerCaseConfig, ScammerCaseReport, ScammerCaseSummary,
    TokenMovementEvidence, TransactionEvidence,
};

pub struct ScammerCaseAnalyzer {
    provider: ProcessedTxProvider,
}

impl ScammerCaseAnalyzer {
    pub fn new(reth_datadir: impl AsRef<str>) -> Result<Self> {
        let provider = ProcessedTxProvider::new(reth_datadir.as_ref())
            .wrap_err("failed to initialize ProcessedTxProvider")?;
        Ok(Self { provider })
    }

    pub async fn analyze_case_file(
        path: impl AsRef<Path>,
        reth_datadir: impl AsRef<str>,
    ) -> Result<ScammerCaseReport> {
        let config = load_case_config(path)?;
        Self::new(reth_datadir)?.analyze_config(config).await
    }

    pub async fn analyze_config(&self, config: ScammerCaseConfig) -> Result<ScammerCaseReport> {
        let suspect = parse_address(&config.suspect_address)?;
        let forwarder = config
            .forwarder_address
            .as_deref()
            .map(parse_address)
            .transpose()?;
        let mut transactions = Vec::new();

        for (label, tx_hash) in config.ordered_transactions() {
            let hash = parse_hash(&tx_hash)?;
            let processed = self
                .provider
                .process_transaction_by_hash(hash)
                .await
                .wrap_err_with(|| format!("failed to process transaction {tx_hash}"))?;
            transactions.push(transaction_evidence(label, &processed, forwarder)?);
        }

        transactions.sort_by_key(|tx| (tx.block_number, tx.tx_index));
        let summary = summarize_report(&transactions, suspect);

        Ok(ScammerCaseReport {
            config,
            summary,
            transactions,
        })
    }
}

pub fn load_case_config(path: impl AsRef<Path>) -> Result<ScammerCaseConfig> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read scammer analytics case {}", path.display()))?;
    toml::from_str(&contents)
        .wrap_err_with(|| format!("failed to parse scammer analytics case {}", path.display()))
}

fn transaction_evidence(
    label: String,
    tx: &ProcessedTransaction,
    forwarder: Option<Address>,
) -> Result<TransactionEvidence> {
    let flows = extract_fund_flows_from_processed_tx(tx)?;
    let gas_cost = tx.fees.gas_price * U256::from(tx.fees.gas_used);
    let forwarder_trace = detect_forwarder_trace(tx, forwarder);

    Ok(TransactionEvidence {
        label,
        tx_hash: hash_string(tx.hash),
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_index: tx.tx_index,
        status: tx.status,
        nonce: tx.nonce,
        from_address: address_string(tx.from_address),
        to_address: tx.to_address.map(address_string),
        contract_address: tx.contract_address.map(address_string),
        value_wei: tx.value.to_string(),
        value_eth: wei_to_eth(tx.value),
        gas_cost_eth: wei_to_eth(gas_cost),
        tx_type: tx.tx_type.clone(),
        actions: tx.actions.clone(),
        eth_movements: flows
            .eth_movements
            .iter()
            .map(|movement| EthMovementEvidence {
                from_address: address_string(movement.from),
                to_address: address_string(movement.to),
                amount_wei: movement.amount.to_string(),
                amount_eth: wei_to_eth(movement.amount),
                movement_type: format!("{:?}", movement.movement_type),
            })
            .collect(),
        token_movements: flows
            .token_movements
            .iter()
            .map(|movement| TokenMovementEvidence {
                token_address: address_string(movement.token_address),
                from_address: address_string(movement.from),
                to_address: address_string(movement.to),
                amount_raw: movement.amount.to_string(),
            })
            .collect(),
        forwarder_trace,
    })
}

fn detect_forwarder_trace(
    tx: &ProcessedTransaction,
    expected_forwarder: Option<Address>,
) -> Option<ForwarderTrace> {
    let forwarder = expected_forwarder?;
    if tx.to_address != Some(forwarder) || tx.value.is_zero() {
        return None;
    }

    let create = tx.internal_transactions.iter().find(|internal| {
        internal.error.is_none()
            && internal.from_address == forwarder
            && internal.value == tx.value
            && internal.trace_type.to_ascii_uppercase().contains("CREATE")
            && internal.to_address.is_some()
    });

    if let Some(create) = create {
        let ephemeral = create.to_address?;
        if let Some(destruct) = tx.internal_transactions.iter().find(|internal| {
            internal.error.is_none()
                && internal.from_address == ephemeral
                && internal.value == tx.value
                && internal.to_address.is_some()
        }) {
            let create_trace_type = create.trace_type.to_ascii_uppercase();
            let pattern = if create_trace_type.contains("CREATE2") {
                "create2_selfdestruct_forward"
            } else {
                "create_selfdestruct_forward"
            };
            return Some(ForwarderTrace {
                pattern: pattern.to_string(),
                forwarder_address: address_string(forwarder),
                ephemeral_address: Some(address_string(ephemeral)),
                sink_address: address_string(destruct.to_address?),
                amount_wei: tx.value.to_string(),
                amount_eth: wei_to_eth(tx.value),
                evidence: vec![
                    format!(
                        "{} creates ephemeral {} with {} wei",
                        address_string(forwarder),
                        address_string(ephemeral),
                        tx.value
                    ),
                    format!(
                        "{} forwards/selfdestructs into {}",
                        address_string(ephemeral),
                        address_string(destruct.to_address?)
                    ),
                ],
            });
        }
    }

    let final_internal = tx
        .internal_transactions
        .iter()
        .filter(|internal| internal.error.is_none() && internal.value == tx.value)
        .filter(|internal| internal.to_address.is_some())
        .last()?;

    Some(ForwarderTrace {
        pattern: "internal_forward".to_string(),
        forwarder_address: address_string(forwarder),
        ephemeral_address: None,
        sink_address: address_string(final_internal.to_address?),
        amount_wei: tx.value.to_string(),
        amount_eth: wei_to_eth(tx.value),
        evidence: vec![format!(
            "last internal value transfer sends {} wei to {}",
            tx.value,
            address_string(final_internal.to_address?)
        )],
    })
}

fn summarize_report(transactions: &[TransactionEvidence], suspect: Address) -> ScammerCaseSummary {
    let suspect = address_string(suspect);
    let mut summary = ScammerCaseSummary {
        transaction_count: transactions.len(),
        failed_transaction_count: transactions.iter().filter(|tx| !tx.status).count(),
        ..ScammerCaseSummary::default()
    };
    let mut forwarder_sinks = BTreeSet::new();
    let mut token_contracts = BTreeSet::new();

    for tx in transactions {
        for movement in &tx.eth_movements {
            if movement.movement_type != "Direct" {
                continue;
            }
            if movement.to_address.eq_ignore_ascii_case(&suspect)
                && !movement.from_address.eq_ignore_ascii_case(&suspect)
            {
                summary.total_creator_direct_eth_in += movement.amount_eth;
            }
            if movement.from_address.eq_ignore_ascii_case(&suspect)
                && !movement.to_address.eq_ignore_ascii_case(&suspect)
            {
                summary.total_creator_direct_eth_out += movement.amount_eth;
            }
        }

        for movement in &tx.token_movements {
            token_contracts.insert(movement.token_address.clone());
        }

        if let Some(trace) = &tx.forwarder_trace {
            summary.forwarder_trace_count += 1;
            summary.total_forwarded_eth += trace.amount_eth;
            forwarder_sinks.insert(trace.sink_address.clone());
        }
    }

    summary.unique_forwarder_sinks = forwarder_sinks.len();
    summary.unique_token_contracts_touched = token_contracts.len();
    summary
}

pub fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| eyre!("invalid Ethereum address {value}: {error}"))
}

pub fn parse_hash(value: &str) -> Result<B256> {
    B256::from_str(value.trim_start_matches("0x"))
        .map_err(|error| eyre!("invalid transaction hash {value}: {error}"))
}

pub fn address_string(address: Address) -> String {
    format!("{address:?}")
}

pub fn hash_string(hash: B256) -> String {
    format!("{hash:?}")
}

pub fn wei_to_eth(value: U256) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18
}
