use alloy_primitives::U256;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub type DecimalAmount = Decimal;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Amount {
    pub raw: U256,
    pub decimals: u8,
}

impl Amount {
    pub const fn zero(decimals: u8) -> Self {
        Self {
            raw: U256::ZERO,
            decimals,
        }
    }

    /// Convert raw amount to decimal value.
    /// E.g. `Amount { raw: 1_000_000_000_000_000_000, decimals: 18 }` → `1.0`
    pub fn to_decimal(&self) -> DecimalAmount {
        let s = self.raw.to_string();
        let decimals = self.decimals as usize;

        // Work via string to avoid Decimal overflow on large raw values.
        // Insert decimal point `decimals` positions from the right.
        if decimals == 0 {
            return DecimalAmount::from_str_exact(&s).unwrap_or_default();
        }

        let mut chars: Vec<char> = s.chars().collect();
        if chars.len() <= decimals {
            // Pad with leading zeros: e.g., raw=123, decimals=5 → "0.00123"
            let pad_len = decimals - chars.len() + 1;
            let mut padded = vec!['0'; pad_len];
            padded.extend(chars);
            chars = padded;
        }

        let decimal_pos = chars.len() - decimals;
        chars.insert(decimal_pos, '.');
        let decimal_str: String = chars.iter().collect();
        DecimalAmount::from_str_exact(&decimal_str).unwrap_or_default()
    }

    /// Convert a decimal value to raw amount.
    /// E.g. `DecimalAmount::from_str_exact("1.0").unwrap()` with `decimals = 18` →
    /// `Amount { raw: 1_000_000_000_000_000_000, decimals: 18 }`
    pub fn from_decimal(dec: DecimalAmount, decimals: u8) -> Self {
        // Work via string to avoid Decimal overflow on large values.
        let s = dec.normalize().to_string();
        let (int_part, frac_part) = if let Some(dot) = s.find('.') {
            (&s[..dot], &s[dot + 1..])
        } else {
            (&s[..], "")
        };

        let frac_trimmed = if frac_part.len() > decimals as usize {
            // Truncate excess fractional digits.
            &frac_part[..decimals as usize]
        } else {
            frac_part
        };

        let zeros_to_add = decimals as usize - frac_trimmed.len();
        let raw_str = format!("{}{}{}", int_part, frac_trimmed, "0".repeat(zeros_to_add));
        let raw = U256::from_str_radix(&raw_str, 10).unwrap_or(U256::ZERO);
        Self { raw, decimals }
    }
}
