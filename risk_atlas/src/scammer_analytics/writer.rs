use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use eyre::{Result, WrapErr};

use super::model::{ScammerCaseReport, TransactionEvidence};

impl ScammerCaseReport {
    pub fn write_artifacts(&self, case_dir: impl AsRef<Path>) -> Result<()> {
        let case_dir = case_dir.as_ref();
        let tx_dir = case_dir.join("artifacts").join("tx_fund_flow");
        let traces_dir = case_dir.join("artifacts").join("traces");
        let reports_dir = case_dir.join("reports");

        fs::create_dir_all(&tx_dir)?;
        fs::create_dir_all(&traces_dir)?;
        fs::create_dir_all(&reports_dir)?;

        write_json(
            tx_dir.join("transactions.json"),
            &self.transactions,
            "transactions",
        )?;
        write_json(
            traces_dir.join("forwarder_traces.json"),
            &self
                .transactions
                .iter()
                .filter_map(|tx| tx.forwarder_trace.as_ref())
                .collect::<Vec<_>>(),
            "forwarder traces",
        )?;
        write_edges_csv(tx_dir.join("eth_edges.csv"), &self.transactions)?;
        write_report_markdown(reports_dir.join("evidence_packet.md"), self)?;

        Ok(())
    }
}

fn write_json<T: serde::Serialize>(path: impl AsRef<Path>, value: &T, label: &str) -> Result<()> {
    let path = path.as_ref();
    let contents = serde_json::to_string_pretty(value)
        .wrap_err_with(|| format!("failed to serialize {label}"))?;
    fs::write(path, contents).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn write_edges_csv(path: impl AsRef<Path>, transactions: &[TransactionEvidence]) -> Result<()> {
    let path = path.as_ref();
    let file =
        File::create(path).wrap_err_with(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "tx_hash,label,block_number,from_address,to_address,amount_eth,movement_type"
    )?;
    for tx in transactions {
        for movement in &tx.eth_movements {
            writeln!(
                writer,
                "{},{},{},{},{},{:.18},{}",
                tx.tx_hash,
                csv_escape(&tx.label),
                tx.block_number,
                movement.from_address,
                movement.to_address,
                movement.amount_eth,
                movement.movement_type
            )?;
        }
    }
    Ok(())
}

fn write_report_markdown(path: impl AsRef<Path>, report: &ScammerCaseReport) -> Result<()> {
    let path = path.as_ref();
    let file =
        File::create(path).wrap_err_with(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "# {}", report.config.title)?;
    writeln!(writer)?;
    writeln!(writer, "Status: `{}`", report.config.status)?;
    writeln!(writer)?;
    writeln!(writer, "## Scope")?;
    writeln!(writer)?;
    writeln!(writer, "- case: `{}`", report.config.case_id)?;
    writeln!(writer, "- chain: `{}`", report.config.chain)?;
    writeln!(writer, "- suspect: `{}`", report.config.suspect_address)?;
    if let Some(token) = &report.config.token_address {
        writeln!(writer, "- token: `{token}`")?;
    }
    if let Some(pool) = &report.config.pool_address {
        writeln!(writer, "- pool: `{pool}`")?;
    }
    if let Some(forwarder) = &report.config.forwarder_address {
        writeln!(writer, "- forwarder: `{forwarder}`")?;
    }

    writeln!(writer)?;
    writeln!(writer, "## Summary")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "- transactions processed: `{}`",
        report.summary.transaction_count
    )?;
    writeln!(
        writer,
        "- forwarder traces: `{}`",
        report.summary.forwarder_trace_count
    )?;
    writeln!(
        writer,
        "- unique forwarder sinks: `{}`",
        report.summary.unique_forwarder_sinks
    )?;
    writeln!(
        writer,
        "- total forwarded ETH: `{:.9}`",
        report.summary.total_forwarded_eth
    )?;
    writeln!(
        writer,
        "- token contracts touched: `{}`",
        report.summary.unique_token_contracts_touched
    )?;

    writeln!(writer)?;
    writeln!(writer, "## Transactions")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "| Block | Label | Tx | From | To | ETH value | Gas ETH |"
    )?;
    writeln!(writer, "| ---: | --- | --- | --- | --- | ---: | ---: |")?;
    for tx in &report.transactions {
        writeln!(
            writer,
            "| {} | `{}` | `{}` | `{}` | `{}` | `{:.9}` | `{:.9}` |",
            tx.block_number,
            tx.label,
            short_hash(&tx.tx_hash),
            short_addr(&tx.from_address),
            tx.to_address
                .as_deref()
                .map(short_addr)
                .unwrap_or_else(|| "contract creation".to_string()),
            tx.value_eth,
            tx.gas_cost_eth
        )?;
    }

    writeln!(writer)?;
    writeln!(writer, "## Forwarder Sinks")?;
    writeln!(writer)?;
    writeln!(writer, "| Tx | Sink | Amount ETH | Pattern |")?;
    writeln!(writer, "| --- | --- | ---: | --- |")?;
    for tx in &report.transactions {
        if let Some(trace) = &tx.forwarder_trace {
            writeln!(
                writer,
                "| `{}` | `{}` | `{:.9}` | `{}` |",
                short_hash(&tx.tx_hash),
                trace.sink_address,
                trace.amount_eth,
                trace.pattern
            )?;
        }
    }

    writeln!(writer)?;
    writeln!(writer, "## Source Links")?;
    writeln!(writer)?;
    for (label, url) in &report.config.links {
        writeln!(writer, "- {label}: {url}")?;
    }

    Ok(())
}

fn short_hash(value: &str) -> String {
    if value.len() <= 14 {
        return value.to_string();
    }
    format!("{}...{}", &value[..10], &value[value.len() - 6..])
}

fn short_addr(value: &str) -> String {
    if value.len() <= 14 {
        return value.to_string();
    }
    format!("{}...{}", &value[..8], &value[value.len() - 6..])
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
