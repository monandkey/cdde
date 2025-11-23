use crate::standard::StandardAvpCode;
use crate::data_type::{AvpDataType, AvpValue, ParseError};

/// AVP information
#[derive(Debug, Clone)]
pub struct AvpInfo {
    pub code: u32,
    pub name: String,
    pub data_type: AvpDataType,
    pub vendor_id: Option<u32>,
}

/// Dictionary manager for AVP lookup and parsing
pub struct DictionaryManager {
    // Future: Add dynamic dictionary support
}

impl DictionaryManager {
    /// Create new dictionary manager
    pub fn new() -> Self {
        Self {}
    }

    /// Lookup AVP information by code
    pub fn lookup(&self, code: u32) -> Option<AvpInfo> {
        // Try standard dictionary first
        if let Some(std_code) = StandardAvpCode::from_u32(code) {
            return Some(AvpInfo {
                code,
                name: std_code.name().to_string(),
                data_type: std_code.data_type(),
                vendor_id: None,
            });
        }

        // TODO: Add dynamic dictionary lookup
        None
    }

    /// Parse AVP data
    pub fn parse_avp(&self, code: u32, data: &[u8]) -> Result<AvpValue, ParseError> {
        let info = self.lookup(code)
            .ok_or(ParseError::UnknownAvpCode(code))?;
        
        info.data_type.parse(data)
    }
}

impl Default for DictionaryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_standard_avp() {
        let manager = DictionaryManager::new();
        let info = manager.lookup(264).unwrap(); // Origin-Host
        
        assert_eq!(info.code, 264);
        assert_eq!(info.name, "Origin-Host");
        assert_eq!(info.data_type, AvpDataType::DiameterIdentity);
        assert_eq!(info.vendor_id, None);
    }

    #[test]
    fn test_lookup_unknown_avp() {
        let manager = DictionaryManager::new();
        let info = manager.lookup(99999);
        
        assert!(info.is_none());
    }

    #[test]
    fn test_parse_avp() {
        let manager = DictionaryManager::new();
        let data = vec![0x00, 0x00, 0x07, 0xD1]; // 2001
        let result = manager.parse_avp(268, &data).unwrap(); // Result-Code
        
        match result {
            AvpValue::Unsigned32(val) => assert_eq!(val, 2001),
            _ => panic!("Expected Unsigned32"),
        }
    }

    #[test]
    fn test_parse_unknown_avp() {
        let manager = DictionaryManager::new();
        let data = vec![0x00, 0x01];
        let result = manager.parse_avp(99999, &data);
        
        assert!(result.is_err());
    }
}
