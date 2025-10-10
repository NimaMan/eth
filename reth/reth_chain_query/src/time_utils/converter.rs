use chrono::{DateTime, Utc};
/// Block-Time Converter
///
/// Central service for converting between block numbers and timestamps.
/// Uses Reth database for accurate timestamps with fallback to estimation.
use std::sync::Arc;

use super::cache::TimestampCache;
use super::{estimate_block_number, estimate_timestamp};
use crate::Result;
use tx_simulator::TxSimulator;

/// Block timestamp data
#[derive(Debug, Clone, Copy)]
pub struct BlockTimestamp {
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
}

/// Block-time converter with caching
pub struct BlockTimeConverter {
    simulator: Arc<TxSimulator>,
    cache: TimestampCache,
}

impl BlockTimeConverter {
    /// Create new converter
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
        Self {
            simulator,
            cache: TimestampCache::new(10000), // Cache up to 10k entries
        }
    }

    /// Get timestamp for a block (with caching)
    pub async fn block_to_timestamp(&self, block_number: u64) -> Result<DateTime<Utc>> {
        // Check cache first
        if let Some(timestamp) = self.cache.get_timestamp(block_number) {
            return Ok(timestamp);
        }

        // Query from Reth database
        let timestamp = self.fetch_block_timestamp(block_number).await?;

        // Cache the result
        self.cache.insert(block_number, timestamp);

        Ok(timestamp)
    }

    /// Find block number for a given timestamp (binary search)
    pub async fn timestamp_to_block(&self, target_time: DateTime<Utc>) -> Result<u64> {
        // Quick check if we have cached blocks near this time
        if let Some(block) = self.cache.find_block_for_timestamp(target_time) {
            return Ok(block);
        }

        // Binary search through blocks
        let latest_block = self.simulator.get_latest_block()?;
        let block = self
            .binary_search_block(0, latest_block, target_time)
            .await?;

        Ok(block)
    }

    /// Get block range for a time period
    pub async fn get_blocks_for_period(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<(u64, u64)> {
        let start_block = self.timestamp_to_block(start_time).await?;
        let end_block = self.timestamp_to_block(end_time).await?;
        Ok((start_block, end_block))
    }

    /// Get blocks for last N periods
    pub async fn get_blocks_for_last_n_periods(
        &self,
        n: usize,
        period_type: super::PeriodType,
    ) -> Result<Vec<super::PeriodBoundary>> {
        let now = Utc::now();
        let start_time = period_type.subtract_periods(now, n);

        let mut boundaries = Vec::new();
        let mut current = start_time;

        for i in 0..n {
            let next = period_type.next_period(current);
            let start_block = self.timestamp_to_block(current).await?;
            let end_block = self.timestamp_to_block(next.min(now)).await?;

            boundaries.push(super::PeriodBoundary {
                period_index: i,
                start_time: current,
                end_time: next.min(now),
                start_block,
                end_block,
            });

            current = next;
            if current >= now {
                break;
            }
        }

        Ok(boundaries)
    }

    /// Batch fetch timestamps for multiple blocks
    pub async fn batch_block_to_timestamp(
        &self,
        block_numbers: &[u64],
    ) -> Result<Vec<BlockTimestamp>> {
        let mut results = Vec::with_capacity(block_numbers.len());

        for &block_number in block_numbers {
            let timestamp = self.block_to_timestamp(block_number).await?;
            results.push(BlockTimestamp {
                block_number,
                timestamp,
            });
        }

        Ok(results)
    }

    // Private helper methods

    /// Fetch block timestamp from Reth database
    async fn fetch_block_timestamp(&self, block_number: u64) -> Result<DateTime<Utc>> {
        // Try to get from Reth DB via simulator's block header info method
        // Returns (timestamp, gas_limit, gas_used, base_fee)
        match self.simulator.get_block_metadata(block_number) {
            Ok((timestamp, _gas_limit, _gas_used, _base_fee)) => {
                Ok(DateTime::from_timestamp(timestamp as i64, 0)
                    .unwrap_or_else(|| estimate_timestamp(block_number)))
            }
            _ => {
                // Fallback to estimation
                Ok(estimate_timestamp(block_number))
            }
        }
    }

    /// Binary search to find block for timestamp
    async fn binary_search_block(
        &self,
        mut low: u64,
        mut high: u64,
        target: DateTime<Utc>,
    ) -> Result<u64> {
        while low < high {
            let mid = (low + high) / 2;
            let mid_time = self.block_to_timestamp(mid).await?;

            if mid_time < target {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        Ok(low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_timestamp() {
        let block = 1_000_000;
        let estimated = super::estimate_timestamp(block);

        // Should be roughly 12M seconds after genesis
        let expected_seconds =
            super::ETHEREUM_GENESIS_TIMESTAMP + (block * super::AVERAGE_BLOCK_TIME) as i64;
        assert_eq!(estimated.timestamp(), expected_seconds);
    }

    #[test]
    fn test_estimate_block_number() {
        let timestamp =
            DateTime::from_timestamp(super::ETHEREUM_GENESIS_TIMESTAMP + 120_000, 0).unwrap();
        let estimated = super::estimate_block_number(timestamp);

        // Should be roughly 10,000 blocks (120,000 / 12)
        assert_eq!(estimated, 10_000);
    }
}
