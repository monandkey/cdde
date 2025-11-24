use crate::data_type::{AvpDataType, AvpValue, ParseError};
use crate::standard::StandardAvpCode;

/// AVP information
#[derive(Debug, Clone)]
pub struct AvpInfo {
    pub code: u32,
    pub name: String,
    pub data_type: AvpDataType,
    pub vendor_id: Option<u32>,
}

use quick_xml::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;

/// Dictionary manager for AVP lookup and parsing
pub struct DictionaryManager {
    dynamic_avps: RwLock<HashMap<u32, AvpInfo>>,
}

#[derive(Debug, Deserialize)]
struct DictionaryXml {
    #[serde(rename = "avp", default)]
    avps: Vec<AvpXml>,
}

#[derive(Debug, Deserialize)]
struct AvpXml {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@code")]
    code: u32,
    #[serde(rename = "@type")]
    data_type: String,
    #[serde(rename = "@vendor-id")]
    vendor_id: Option<u32>,
}

impl DictionaryManager {
    /// Create new dictionary manager
    pub fn new() -> Self {
        Self {
            dynamic_avps: RwLock::new(HashMap::new()),
        }
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

        // Try dynamic dictionary
        if let Ok(guard) = self.dynamic_avps.read() {
            if let Some(info) = guard.get(&code) {
                return Some(info.clone());
            }
        }

        None
    }

    /// Parse AVP data
    pub fn parse_avp(&self, code: u32, data: &[u8]) -> Result<AvpValue, ParseError> {
        let info = self.lookup(code).ok_or(ParseError::UnknownAvpCode(code))?;

        info.data_type.parse(data)
    }

    /// Load dynamic dictionary from XML string
    pub fn load_dynamic_dictionary(&self, xml: &str) -> Result<(), String> {
        let dict: DictionaryXml = from_str(xml).map_err(|e| e.to_string())?;

        let mut guard = self
            .dynamic_avps
            .write()
            .map_err(|_| "Lock poisoned".to_string())?;

        for avp in dict.avps {
            let data_type = match avp.data_type.as_str() {
                "OctetString" => AvpDataType::OctetString,
                "Integer32" => AvpDataType::Integer32,
                "Integer64" => AvpDataType::Integer64,
                "Unsigned32" => AvpDataType::Unsigned32,
                "Unsigned64" => AvpDataType::Unsigned64,
                "Float32" => AvpDataType::Float32,
                "Float64" => AvpDataType::Float64,
                "Grouped" => AvpDataType::Grouped,
                "Address" => AvpDataType::Address,
                "Time" => AvpDataType::Time,
                "UTF8String" => AvpDataType::Utf8String,
                "DiameterIdentity" => AvpDataType::DiameterIdentity,
                "DiameterURI" => AvpDataType::DiameterUri,
                "Enumerated" => AvpDataType::Enumerated,
                "IPFilterRule" => AvpDataType::IpFilterRule,
                _ => continue, // Skip unknown types or handle error
            };

            let info = AvpInfo {
                code: avp.code,
                name: avp.name,
                data_type,
                vendor_id: avp.vendor_id,
            };

            guard.insert(avp.code, info);
        }

        Ok(())
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

    #[test]
    fn test_load_dynamic_dictionary() {
        let manager = DictionaryManager::new();
        let xml = r#"
        <dictionary>
            <avp name="Test-AVP" code="10001" type="Unsigned32" vendor-id="9999"/>
        </dictionary>
        "#;

        manager
            .load_dynamic_dictionary(xml)
            .expect("Failed to load dictionary");

        let info = manager.lookup(10001).unwrap();
        assert_eq!(info.name, "Test-AVP");
        assert_eq!(info.data_type, AvpDataType::Unsigned32);
        assert_eq!(info.vendor_id, Some(9999));
    }
}
