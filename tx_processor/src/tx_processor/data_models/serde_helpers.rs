use alloy_primitives::Address;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
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

pub fn serialize_i128_to_string<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        serializer.serialize_str(&value.to_string())
    } else {
        serializer.serialize_i128(*value)
    }
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

pub fn serialize_u128_to_string<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        serializer.serialize_str(&value.to_string())
    } else {
        serializer.serialize_u128(*value)
    }
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

pub mod json_value_vec_map_binary {
    use super::*;

    pub fn serialize<S>(
        value: &[HashMap<String, serde_json::Value>],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            return value.serialize(serializer);
        }

        let encoded = value
            .iter()
            .map(|map| {
                map.iter()
                    .map(|(key, value)| {
                        serde_json::to_vec(value)
                            .map(|bytes| (key.clone(), bytes))
                            .map_err(serde::ser::Error::custom)
                    })
                    .collect::<Result<HashMap<_, _>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        encoded.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            return Vec::<HashMap<String, serde_json::Value>>::deserialize(deserializer);
        }

        let encoded = Vec::<HashMap<String, Vec<u8>>>::deserialize(deserializer)?;
        encoded
            .into_iter()
            .map(|map| {
                map.into_iter()
                    .map(|(key, bytes)| {
                        serde_json::from_slice(&bytes)
                            .map(|value| (key, value))
                            .map_err(serde::de::Error::custom)
                    })
                    .collect::<Result<HashMap<_, _>, _>>()
            })
            .collect()
    }
}

pub mod json_value_address_map_binary {
    use super::*;

    pub fn serialize<S>(
        value: &HashMap<Address, serde_json::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            return value.serialize(serializer);
        }

        let encoded = value
            .iter()
            .map(|(key, value)| {
                serde_json::to_vec(value)
                    .map(|bytes| (*key, bytes))
                    .map_err(serde::ser::Error::custom)
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        encoded.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<Address, serde_json::Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            return HashMap::<Address, serde_json::Value>::deserialize(deserializer);
        }

        let encoded = HashMap::<Address, Vec<u8>>::deserialize(deserializer)?;
        encoded
            .into_iter()
            .map(|(key, bytes)| {
                serde_json::from_slice(&bytes)
                    .map(|value| (key, value))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

pub mod option_json_value_vec_map_binary {
    use super::*;

    pub fn serialize<S>(
        value: &Option<Vec<HashMap<String, serde_json::Value>>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            return value.serialize(serializer);
        }

        let encoded = value
            .as_ref()
            .map(|items| {
                items
                    .iter()
                    .map(|map| {
                        map.iter()
                            .map(|(key, value)| {
                                serde_json::to_vec(value)
                                    .map(|bytes| (key.clone(), bytes))
                                    .map_err(serde::ser::Error::custom)
                            })
                            .collect::<Result<HashMap<_, _>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        encoded.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<Vec<HashMap<String, serde_json::Value>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            return Option::<Vec<HashMap<String, serde_json::Value>>>::deserialize(deserializer);
        }

        let encoded = Option::<Vec<HashMap<String, Vec<u8>>>>::deserialize(deserializer)?;
        encoded
            .map(|items| {
                items
                    .into_iter()
                    .map(|map| {
                        map.into_iter()
                            .map(|(key, bytes)| {
                                serde_json::from_slice(&bytes)
                                    .map(|value| (key, value))
                                    .map_err(serde::de::Error::custom)
                            })
                            .collect::<Result<HashMap<_, _>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
    }
}

pub mod option_json_value_address_map_binary {
    use super::*;

    pub fn serialize<S>(
        value: &Option<HashMap<Address, serde_json::Value>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            return value.serialize(serializer);
        }

        let encoded = value
            .as_ref()
            .map(|map| {
                map.iter()
                    .map(|(key, value)| {
                        serde_json::to_vec(value)
                            .map(|bytes| (*key, bytes))
                            .map_err(serde::ser::Error::custom)
                    })
                    .collect::<Result<HashMap<_, _>, _>>()
            })
            .transpose()?;
        encoded.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<HashMap<Address, serde_json::Value>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            return Option::<HashMap<Address, serde_json::Value>>::deserialize(deserializer);
        }

        let encoded = Option::<HashMap<Address, Vec<u8>>>::deserialize(deserializer)?;
        encoded
            .map(|map| {
                map.into_iter()
                    .map(|(key, bytes)| {
                        serde_json::from_slice(&bytes)
                            .map(|value| (key, value))
                            .map_err(serde::de::Error::custom)
                    })
                    .collect::<Result<HashMap<_, _>, _>>()
            })
            .transpose()
    }
}

pub mod option_bytes_hex {
    use super::*;

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !serializer.is_human_readable() {
            return value.serialize(serializer);
        }

        match value {
            Some(bytes) => serializer.serialize_some(&format!("0x{}", hex::encode(bytes))),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if !deserializer.is_human_readable() {
            return Option::<Vec<u8>>::deserialize(deserializer);
        }

        let value = Option::<String>::deserialize(deserializer)?;
        value
            .map(|value| {
                let value = value.strip_prefix("0x").unwrap_or(&value);
                hex::decode(value).map_err(serde::de::Error::custom)
            })
            .transpose()
    }
}
