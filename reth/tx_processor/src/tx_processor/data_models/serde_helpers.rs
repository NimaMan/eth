use serde::de::{self, Deserializer, Visitor};
use std::fmt;

/// Deserialize an `i128` from either a JSON number or a decimal/hex string.
pub fn deserialize_i128_from_any<'de, D>(deserializer: D) -> Result<i128, D::Error>
where
    D: Deserializer<'de>,
{
    struct I128Visitor;

    impl<'de> Visitor<'de> for I128Visitor {
        type Value = i128;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer or string representing a signed 128-bit value")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as i128)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            i128::try_from(value)
                .map_err(|_| de::Error::custom("unsigned value out of range for i128"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            parse_i128(value).map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(I128Visitor)
}

/// Deserialize a `u128` from either a JSON number or a decimal/hex string.
pub fn deserialize_u128_from_any<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    struct U128Visitor;

    impl<'de> Visitor<'de> for U128Visitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer or string representing an unsigned 128-bit value")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as u128)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value < 0 {
                return Err(de::Error::custom("negative value not allowed for u128"));
            }
            Ok(value as u128)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            parse_u128(value).map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(U128Visitor)
}

fn parse_i128(value: &str) -> Result<i128, String> {
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        let magnitude = i128::from_str_radix(hex, 16)
            .map_err(|err| format!("failed to parse negative hex i128: {err}"))?;
        return Ok(-magnitude);
    }

    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i128::from_str_radix(hex, 16)
            .map_err(|err| format!("failed to parse hex i128: {err}"));
    }

    value
        .parse::<i128>()
        .map_err(|err| format!("failed to parse decimal i128: {err}"))
}

fn parse_u128(value: &str) -> Result<u128, String> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return u128::from_str_radix(hex, 16)
            .map_err(|err| format!("failed to parse hex u128: {err}"));
    }

    value
        .parse::<u128>()
        .map_err(|err| format!("failed to parse decimal u128: {err}"))
}
