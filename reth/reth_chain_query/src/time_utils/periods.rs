/// Time Period Management
/// 
/// Defines time periods and boundaries for aggregation operations.

use chrono::{DateTime, Duration, Utc, Datelike, Timelike};
use serde::{Serialize, Deserialize};

/// Time period types for aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeriodType {
    Block,      // Individual blocks
    Minute,     // 5 blocks (~1 minute)
    Hour,       // 300 blocks (~1 hour)
    FourHour,   // 1200 blocks (~4 hours)
    Day,        // 7200 blocks (~24 hours)
    Week,       // 50400 blocks (~7 days)
    Month,      // 216000 blocks (~30 days)
    Quarter,    // 648000 blocks (~90 days)
    Year,       // 2628000 blocks (~365 days)
}

impl PeriodType {
    /// Get number of blocks in this period (assuming 12 second blocks)
    /// 
    /// Note: These are theoretical values based on 12-second average block time.
    /// Actual Ethereum block times vary (typically 12-14 seconds), so real-world
    /// period boundaries may contain slightly different block counts.
    /// For example:
    /// - Theoretical hourly: 300 blocks
    /// - Actual observed: 250-350 blocks depending on network conditions
    pub fn blocks_per_period(&self) -> u64 {
        match self {
            PeriodType::Block => 1,
            PeriodType::Minute => 5,
            PeriodType::Hour => 300,
            PeriodType::FourHour => 1200,
            PeriodType::Day => 7200,
            PeriodType::Week => 50400,
            PeriodType::Month => 216000,
            PeriodType::Quarter => 648000,
            PeriodType::Year => 2628000,
        }
    }
    
    /// Get approximate duration of this period
    pub fn duration(&self) -> Duration {
        Duration::seconds((self.blocks_per_period() * super::AVERAGE_BLOCK_TIME) as i64)
    }
    
    /// Get period name as string
    pub fn as_str(&self) -> &str {
        match self {
            PeriodType::Block => "block",
            PeriodType::Minute => "minute",
            PeriodType::Hour => "hour",
            PeriodType::FourHour => "4hour",
            PeriodType::Day => "day",
            PeriodType::Week => "week",
            PeriodType::Month => "month",
            PeriodType::Quarter => "quarter",
            PeriodType::Year => "year",
        }
    }
    
    /// Round a timestamp down to the start of this period
    pub fn floor_timestamp(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            PeriodType::Block => timestamp, // No rounding for blocks
            PeriodType::Minute => {
                timestamp
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::Hour => {
                timestamp
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::FourHour => {
                let hour = (timestamp.hour() / 4) * 4;
                timestamp
                    .with_hour(hour).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::Day => {
                timestamp
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::Week => {
                // Start of week (Monday)
                let days_since_monday = timestamp.weekday().num_days_from_monday();
                timestamp
                    .checked_sub_signed(Duration::days(days_since_monday as i64))
                    .unwrap()
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::Month => {
                timestamp
                    .with_day(1).unwrap()
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::Quarter => {
                let month = ((timestamp.month() - 1) / 3) * 3 + 1;
                timestamp
                    .with_month(month).unwrap()
                    .with_day(1).unwrap()
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
            PeriodType::Year => {
                timestamp
                    .with_month(1).unwrap()
                    .with_day(1).unwrap()
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
            }
        }
    }
    
    /// Get the next period boundary
    pub fn next_period(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        let floored = self.floor_timestamp(timestamp);
        floored + self.duration()
    }
    
    /// Subtract N periods from a timestamp
    pub fn subtract_periods(&self, timestamp: DateTime<Utc>, n: usize) -> DateTime<Utc> {
        timestamp - self.duration() * n as i32
    }
}

/// Time period with block boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePeriod {
    pub period_type: PeriodType,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub start_block: u64,
    pub end_block: u64,
}

impl TimePeriod {
    /// Create a new time period
    pub fn new(
        period_type: PeriodType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        start_block: u64,
        end_block: u64,
    ) -> Self {
        Self {
            period_type,
            start_time,
            end_time,
            start_block,
            end_block,
        }
    }
    
    /// Get the duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        (self.end_time - self.start_time).num_seconds()
    }
    
    /// Get the number of blocks in this period
    pub fn block_count(&self) -> u64 {
        self.end_block - self.start_block
    }
    
    /// Check if a block is within this period
    pub fn contains_block(&self, block: u64) -> bool {
        block >= self.start_block && block <= self.end_block
    }
    
    /// Check if a timestamp is within this period
    pub fn contains_timestamp(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start_time && timestamp <= self.end_time
    }
}

/// Period boundary for iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodBoundary {
    pub period_index: usize,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub start_block: u64,
    pub end_block: u64,
}

impl PeriodBoundary {
    /// Convert to TimePeriod
    pub fn to_time_period(&self, period_type: PeriodType) -> TimePeriod {
        TimePeriod::new(
            period_type,
            self.start_time,
            self.end_time,
            self.start_block,
            self.end_block,
        )
    }
}

/// Generate period boundaries for a range
pub fn generate_period_boundaries(
    period_type: PeriodType,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut boundaries = Vec::new();
    let mut current = period_type.floor_timestamp(start_time);
    
    while current < end_time {
        let next = period_type.next_period(current);
        boundaries.push((
            current.max(start_time),
            next.min(end_time)
        ));
        current = next;
    }
    
    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_period_blocks() {
        assert_eq!(PeriodType::Hour.blocks_per_period(), 300);
        assert_eq!(PeriodType::Day.blocks_per_period(), 7200);
        assert_eq!(PeriodType::Week.blocks_per_period(), 50400);
    }
    
    #[test]
    fn test_floor_timestamp() {
        let timestamp = DateTime::parse_from_rfc3339("2024-01-15T14:35:45Z")
            .unwrap()
            .with_timezone(&Utc);
        
        let hour_floor = PeriodType::Hour.floor_timestamp(timestamp);
        assert_eq!(hour_floor.hour(), 14);
        assert_eq!(hour_floor.minute(), 0);
        assert_eq!(hour_floor.second(), 0);
        
        let day_floor = PeriodType::Day.floor_timestamp(timestamp);
        assert_eq!(day_floor.hour(), 0);
        assert_eq!(day_floor.minute(), 0);
        assert_eq!(day_floor.day(), 15);
    }
}