use super::{
    AddressPoolPnlSummary, AddressPoolPosition, PnlAddressPositionExport, PnlConservationExport,
    PnlMovementExport, PnlPoolExport, PoolPnlEntry, PoolPnlTracker, TokenPnlTracker,
};

impl TokenPnlTracker {
    pub fn export_pools(
        &self,
        protocol: Option<&str>,
        mark_price_denom_per_token: Option<f64>,
    ) -> Vec<PnlPoolExport> {
        self.pools()
            .map(|(_, pool)| pool.export(protocol, mark_price_denom_per_token))
            .collect()
    }
}

impl PoolPnlTracker {
    pub fn export(
        &self,
        protocol: Option<&str>,
        mark_price_denom_per_token: Option<f64>,
    ) -> PnlPoolExport {
        let summaries = self.address_summaries(mark_price_denom_per_token);
        let address_positions = self
            .positions
            .values()
            .zip(summaries)
            .map(|(position, summary)| export_address_position(position, summary))
            .collect();

        let movements = self
            .recent_entries
            .iter()
            .enumerate()
            .map(|(entry_index, entry)| export_movement(entry_index as u64, entry))
            .collect();

        PnlPoolExport {
            pool_id: self.pool_address.clone(),
            token_address: self.token_address.clone(),
            denom_address: self.denom_address.clone(),
            protocol: protocol.map(str::to_string),
            token_decimals: self.token_decimals,
            denom_decimals: self.denom_decimals,
            tx_count: self.tx_count,
            latest_block_number: self.latest_block_number,
            latest_block_timestamp: self.latest_block_timestamp,
            conservation: PnlConservationExport {
                token_in_raw: self.conservation.token_in_raw.to_string(),
                token_out_raw: self.conservation.token_out_raw.to_string(),
                denom_in_raw: self.conservation.denom_in_raw.to_string(),
                denom_out_raw: self.conservation.denom_out_raw.to_string(),
                pool_token_in_raw: self.conservation.pool_token_in_raw.to_string(),
                pool_token_out_raw: self.conservation.pool_token_out_raw.to_string(),
                pool_denom_in_raw: self.conservation.pool_denom_in_raw.to_string(),
                pool_denom_out_raw: self.conservation.pool_denom_out_raw.to_string(),
                native_fee_raw: self.conservation.native_fee_raw.to_string(),
                native_bribe_raw: self.conservation.native_bribe_raw.to_string(),
                token_transfer_count: self.conservation.token_transfer_count,
                denom_transfer_count: self.conservation.denom_transfer_count,
            },
            address_positions,
            movements,
        }
    }
}

fn export_address_position(
    position: &AddressPoolPosition,
    summary: AddressPoolPnlSummary,
) -> PnlAddressPositionExport {
    PnlAddressPositionExport {
        address: position.address.clone(),
        token_in_raw: position.token_in_raw.to_string(),
        token_out_raw: position.token_out_raw.to_string(),
        denom_in_raw: position.denom_in_raw.to_string(),
        denom_out_raw: position.denom_out_raw.to_string(),
        native_fee_raw: position.native_fee_raw.to_string(),
        native_bribe_raw: position.native_bribe_raw.to_string(),
        first_block: position.first_block,
        latest_block: position.latest_block,
        movement_count: position.movement_count,
        token_balance_raw: summary.token_balance_raw,
        denom_cashflow_raw: summary.denom_cashflow_raw,
        token_balance: summary.token_balance,
        denom_cashflow: summary.denom_cashflow,
        native_fee: summary.native_fee,
        native_bribe: summary.native_bribe,
        marked_token_value_denom: summary.marked_token_value_denom,
        pnl_proxy_denom: summary.pnl_proxy_denom,
    }
}

fn export_movement(entry_index: u64, entry: &PoolPnlEntry) -> PnlMovementExport {
    PnlMovementExport {
        entry_index,
        tx_hash: entry.tx_hash.clone(),
        block_number: entry.block_number,
        block_timestamp: entry.block_timestamp,
        tx_index: entry.tx_index,
        log_index: entry.log_index,
        address: entry.address.clone(),
        kind: entry.kind.as_str().to_string(),
        token_in_raw: entry.token_in_raw.clone(),
        token_out_raw: entry.token_out_raw.clone(),
        denom_in_raw: entry.denom_in_raw.clone(),
        denom_out_raw: entry.denom_out_raw.clone(),
        native_fee_raw: entry.native_fee_raw.clone(),
        native_bribe_raw: entry.native_bribe_raw.clone(),
        pool_direct: entry.pool_direct,
    }
}
