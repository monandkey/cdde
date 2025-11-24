use thiserror::Error;

/// AVP data type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvpDataType {
    OctetString,
    Utf8String,
    DiameterIdentity,
    DiameterUri,
    Unsigned32,
    Unsigned64,
    Integer32,
    Integer64,
    Float32,
    Float64,
    Grouped,
    Enumerated,
    Time,
    Address,
    IpFilterRule,
}

/// AVP value after parsing
#[derive(Debug, Clone, PartialEq)]
pub enum AvpValue {
    OctetString(Vec<u8>),
    Utf8String(String),
    DiameterIdentity(String),
    DiameterUri(String),
    Unsigned32(u32),
    Unsigned64(u64),
    Integer32(i32),
    Integer64(i64),
    Float32(f32),
    Float64(f64),
    Grouped(Vec<u8>), // Raw grouped AVP data
    Enumerated(i32),
    Time(u32),
    Address(Vec<u8>),
    IpFilterRule(Vec<u8>),
}

/// Parse errors
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid length for data type")]
    InvalidLength,

    #[error("Invalid UTF-8 string")]
    InvalidUtf8,

    #[error("Unknown AVP code: {0}")]
    UnknownAvpCode(u32),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl AvpDataType {
    /// Parse raw bytes into AvpValue according to data type
    pub fn parse(&self, data: &[u8]) -> Result<AvpValue, ParseError> {
        match self {
            Self::OctetString => Ok(AvpValue::OctetString(data.to_vec())),

            Self::Utf8String | Self::DiameterIdentity | Self::DiameterUri => {
                let s = String::from_utf8(data.to_vec()).map_err(|_| ParseError::InvalidUtf8)?;
                match self {
                    Self::Utf8String => Ok(AvpValue::Utf8String(s)),
                    Self::DiameterIdentity => Ok(AvpValue::DiameterIdentity(s)),
                    Self::DiameterUri => Ok(AvpValue::DiameterUri(s)),
                    _ => unreachable!(),
                }
            }

            Self::Unsigned32 => {
                if data.len() != 4 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = u32::from_be_bytes(bytes);
                Ok(AvpValue::Unsigned32(value))
            }

            Self::Unsigned64 => {
                if data.len() != 8 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = u64::from_be_bytes(bytes);
                Ok(AvpValue::Unsigned64(value))
            }

            Self::Integer32 => {
                if data.len() != 4 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = i32::from_be_bytes(bytes);
                Ok(AvpValue::Integer32(value))
            }

            Self::Integer64 => {
                if data.len() != 8 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = i64::from_be_bytes(bytes);
                Ok(AvpValue::Integer64(value))
            }

            Self::Float32 => {
                if data.len() != 4 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = f32::from_be_bytes(bytes);
                Ok(AvpValue::Float32(value))
            }

            Self::Float64 => {
                if data.len() != 8 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = f64::from_be_bytes(bytes);
                Ok(AvpValue::Float64(value))
            }

            Self::Grouped => Ok(AvpValue::Grouped(data.to_vec())),

            Self::Enumerated => {
                if data.len() != 4 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = i32::from_be_bytes(bytes);
                Ok(AvpValue::Enumerated(value))
            }

            Self::Time => {
                if data.len() != 4 {
                    return Err(ParseError::InvalidLength);
                }
                let bytes = data.try_into().map_err(|_| ParseError::InvalidLength)?;
                let value = u32::from_be_bytes(bytes);
                Ok(AvpValue::Time(value))
            }

            Self::Address => Ok(AvpValue::Address(data.to_vec())),

            Self::IpFilterRule => Ok(AvpValue::IpFilterRule(data.to_vec())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unsigned32() {
        let data = vec![0x00, 0x00, 0x07, 0xD1]; // 2001
        let result = AvpDataType::Unsigned32.parse(&data).unwrap();

        match result {
            AvpValue::Unsigned32(val) => assert_eq!(val, 2001),
            _ => panic!("Expected Unsigned32"),
        }
    }

    #[test]
    fn test_parse_utf8_string() {
        let data = b"test.realm.com".to_vec();
        let result = AvpDataType::Utf8String.parse(&data).unwrap();

        match result {
            AvpValue::Utf8String(s) => assert_eq!(s, "test.realm.com"),
            _ => panic!("Expected Utf8String"),
        }
    }

    #[test]
    fn test_parse_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = AvpDataType::Utf8String.parse(&invalid_utf8);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_length() {
        let data = vec![0x00, 0x01]; // Too short for Unsigned32
        let result = AvpDataType::Unsigned32.parse(&data);

        assert!(result.is_err());
    }
}
