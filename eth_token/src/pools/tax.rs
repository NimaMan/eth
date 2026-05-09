use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxBucket {
    Unknown,
    NoTax,
    LowTax,
    ModerateTax,
    HighTax,
    ExtremeTax,
}

impl TaxBucket {
    pub fn from_percent(value: Option<f64>) -> Self {
        let Some(value) = value.filter(|value| value.is_finite() && *value >= 0.0) else {
            return Self::Unknown;
        };

        if value == 0.0 {
            Self::NoTax
        } else if value < 10.0 {
            Self::LowTax
        } else if value <= 20.0 {
            Self::ModerateTax
        } else if value <= 40.0 {
            Self::HighTax
        } else {
            Self::ExtremeTax
        }
    }

    pub fn combined(buy_tax: Option<f64>, sell_tax: Option<f64>) -> Self {
        Self::from_percent(buy_tax).max(Self::from_percent(sell_tax))
    }

    pub fn risk_label(self) -> Option<&'static str> {
        match self {
            Self::HighTax => Some("high_tax"),
            Self::ExtremeTax => Some("extreme_tax"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TaxBucket;

    #[test]
    fn tax_bucket_boundaries_are_explicit() {
        assert_eq!(TaxBucket::from_percent(None), TaxBucket::Unknown);
        assert_eq!(TaxBucket::from_percent(Some(-1.0)), TaxBucket::Unknown);
        assert_eq!(TaxBucket::from_percent(Some(f64::NAN)), TaxBucket::Unknown);
        assert_eq!(TaxBucket::from_percent(Some(0.0)), TaxBucket::NoTax);
        assert_eq!(TaxBucket::from_percent(Some(0.01)), TaxBucket::LowTax);
        assert_eq!(TaxBucket::from_percent(Some(9.99)), TaxBucket::LowTax);
        assert_eq!(TaxBucket::from_percent(Some(10.0)), TaxBucket::ModerateTax);
        assert_eq!(TaxBucket::from_percent(Some(20.0)), TaxBucket::ModerateTax);
        assert_eq!(TaxBucket::from_percent(Some(20.01)), TaxBucket::HighTax);
        assert_eq!(TaxBucket::from_percent(Some(40.0)), TaxBucket::HighTax);
        assert_eq!(TaxBucket::from_percent(Some(40.01)), TaxBucket::ExtremeTax);
    }

    #[test]
    fn combined_tax_bucket_uses_most_severe_known_tax() {
        assert_eq!(
            TaxBucket::combined(Some(0.0), Some(19.0)),
            TaxBucket::ModerateTax
        );
        assert_eq!(TaxBucket::combined(None, Some(42.0)), TaxBucket::ExtremeTax);
        assert_eq!(TaxBucket::combined(None, None), TaxBucket::Unknown);
    }
}
