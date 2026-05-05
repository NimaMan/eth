use eyre::{eyre, Result};

pub fn parse_raw_i128(value: impl AsRef<str>) -> Result<i128> {
    let cleaned = value.as_ref().trim();
    if cleaned.is_empty() {
        return Err(eyre!("empty numeric value"));
    }

    let lowered = cleaned.to_ascii_lowercase();
    if let Some(hex) = lowered.strip_prefix("-0x") {
        return Ok(-i128::from_str_radix(hex, 16)?);
    }
    if let Some(hex) = lowered.strip_prefix("0x") {
        return Ok(i128::from_str_radix(hex, 16)?);
    }

    Ok(cleaned.parse::<i128>()?)
}

pub fn parse_raw_i128_or(value: Option<impl AsRef<str>>, default: i128) -> i128 {
    value
        .and_then(|value| parse_raw_i128(value).ok())
        .unwrap_or(default)
}

pub fn parse_raw_u128(value: impl AsRef<str>) -> Result<u128> {
    let cleaned = value.as_ref().trim();
    if cleaned.is_empty() {
        return Err(eyre!("empty numeric value"));
    }
    if cleaned.starts_with('-') {
        return Err(eyre!("negative value cannot be parsed as u128: {cleaned}"));
    }

    let lowered = cleaned.to_ascii_lowercase();
    if let Some(hex) = lowered.strip_prefix("0x") {
        return Ok(u128::from_str_radix(hex, 16)?);
    }

    Ok(cleaned.parse::<u128>()?)
}

pub fn parse_raw_u128_or(value: Option<impl AsRef<str>>, default: u128) -> u128 {
    value
        .and_then(|value| parse_raw_u128(value).ok())
        .unwrap_or(default)
}

pub fn parse_raw_f64(value: impl AsRef<str>) -> Result<f64> {
    let cleaned = value.as_ref().trim();
    if cleaned.is_empty() {
        return Err(eyre!("empty numeric value"));
    }

    let lowered = cleaned.to_ascii_lowercase();
    if lowered.starts_with("0x") || lowered.starts_with("-0x") {
        return Ok(parse_raw_i128(cleaned)? as f64);
    }

    Ok(cleaned.parse::<f64>()?)
}

pub fn parse_raw_f64_or(value: Option<impl AsRef<str>>, default: f64) -> f64 {
    value
        .and_then(|value| parse_raw_f64(value).ok())
        .unwrap_or(default)
}

pub fn parse_be_bytes_u128(bytes: impl AsRef<[u8]>) -> Result<u128> {
    let bytes = bytes.as_ref();
    if bytes.len() > 16 {
        return Err(eyre!(
            "cannot parse {} big-endian bytes into u128",
            bytes.len()
        ));
    }

    let mut value = 0u128;
    for byte in bytes {
        value = (value << 8) | u128::from(*byte);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_raw_i128_supports_decimal_and_hex() {
        assert_eq!(parse_raw_i128("42").unwrap(), 42);
        assert_eq!(parse_raw_i128("0x2a").unwrap(), 42);
        assert_eq!(parse_raw_i128("-0x2a").unwrap(), -42);
    }

    #[test]
    fn parse_raw_f64_supports_hex_and_defaults() {
        assert_eq!(parse_raw_f64("0x10").unwrap(), 16.0);
        assert_eq!(parse_raw_f64_or(Some(""), 7.5), 7.5);
        assert_eq!(parse_raw_f64_or(Option::<&str>::None, 3.0), 3.0);
    }

    #[test]
    fn parse_be_bytes_u128_reads_big_endian_bytes() {
        assert_eq!(parse_be_bytes_u128([0x01, 0x00]).unwrap(), 256);
    }
}
