use crate::data_type::AvpDataType;

/// Standard AVP Code definitions from RFC 6733 and 3GPP specifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum StandardAvpCode {
    // ========================================
    // RFC 6733 Base Protocol
    // ========================================
    UserName = 1,
    HostIpAddress = 257,
    AuthApplicationId = 258,
    AcctApplicationId = 259,
    VendorSpecificApplicationId = 260,
    SessionId = 263,
    OriginHost = 264,
    SupportedVendorId = 265,
    VendorId = 266,
    FirmwareRevision = 267,
    ResultCode = 268,
    ProductName = 269,
    RouteRecord = 282,
    DestinationRealm = 283,
    DestinationHost = 293,
    OriginRealm = 296,
    
    // ========================================
    // 3GPP S6a (TS 29.272)
    // ========================================
    SubscriptionData = 1400,
    UlrFlags = 1405,
    UlaFlags = 1406,
    VisitedPlmnId = 1407,
    RequestedEutranAuthInfo = 1408,
    
    // ========================================
    // 3GPP Gx (TS 29.212)
    // ========================================
    ChargingRuleInstall = 1001,
    ChargingRuleName = 1005,
    EventTrigger = 1006,
}

impl StandardAvpCode {
    /// Convert u32 code to StandardAvpCode
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::UserName),
            257 => Some(Self::HostIpAddress),
            258 => Some(Self::AuthApplicationId),
            259 => Some(Self::AcctApplicationId),
            260 => Some(Self::VendorSpecificApplicationId),
            263 => Some(Self::SessionId),
            264 => Some(Self::OriginHost),
            265 => Some(Self::SupportedVendorId),
            266 => Some(Self::VendorId),
            267 => Some(Self::FirmwareRevision),
            268 => Some(Self::ResultCode),
            269 => Some(Self::ProductName),
            282 => Some(Self::RouteRecord),
            283 => Some(Self::DestinationRealm),
            293 => Some(Self::DestinationHost),
            296 => Some(Self::OriginRealm),
            1400 => Some(Self::SubscriptionData),
            1405 => Some(Self::UlrFlags),
            1406 => Some(Self::UlaFlags),
            1407 => Some(Self::VisitedPlmnId),
            1408 => Some(Self::RequestedEutranAuthInfo),
            1001 => Some(Self::ChargingRuleInstall),
            1005 => Some(Self::ChargingRuleName),
            1006 => Some(Self::EventTrigger),
            _ => None,
        }
    }

    /// Get AVP name
    pub fn name(&self) -> &'static str {
        match self {
            Self::UserName => "User-Name",
            Self::HostIpAddress => "Host-IP-Address",
            Self::AuthApplicationId => "Auth-Application-Id",
            Self::AcctApplicationId => "Acct-Application-Id",
            Self::VendorSpecificApplicationId => "Vendor-Specific-Application-Id",
            Self::SessionId => "Session-Id",
            Self::OriginHost => "Origin-Host",
            Self::SupportedVendorId => "Supported-Vendor-Id",
            Self::VendorId => "Vendor-Id",
            Self::FirmwareRevision => "Firmware-Revision",
            Self::ResultCode => "Result-Code",
            Self::ProductName => "Product-Name",
            Self::RouteRecord => "Route-Record",
            Self::DestinationRealm => "Destination-Realm",
            Self::DestinationHost => "Destination-Host",
            Self::OriginRealm => "Origin-Realm",
            Self::SubscriptionData => "Subscription-Data",
            Self::UlrFlags => "ULR-Flags",
            Self::UlaFlags => "ULA-Flags",
            Self::VisitedPlmnId => "Visited-PLMN-Id",
            Self::RequestedEutranAuthInfo => "Requested-EUTRAN-Authentication-Info",
            Self::ChargingRuleInstall => "Charging-Rule-Install",
            Self::ChargingRuleName => "Charging-Rule-Name",
            Self::EventTrigger => "Event-Trigger",
        }
    }

    /// Get AVP data type
    pub fn data_type(&self) -> AvpDataType {
        match self {
            Self::UserName => AvpDataType::Utf8String,
            Self::HostIpAddress => AvpDataType::Address,
            Self::AuthApplicationId => AvpDataType::Unsigned32,
            Self::AcctApplicationId => AvpDataType::Unsigned32,
            Self::VendorSpecificApplicationId => AvpDataType::Grouped,
            Self::SessionId => AvpDataType::Utf8String,
            Self::OriginHost => AvpDataType::DiameterIdentity,
            Self::SupportedVendorId => AvpDataType::Unsigned32,
            Self::VendorId => AvpDataType::Unsigned32,
            Self::FirmwareRevision => AvpDataType::Unsigned32,
            Self::ResultCode => AvpDataType::Unsigned32,
            Self::ProductName => AvpDataType::Utf8String,
            Self::RouteRecord => AvpDataType::DiameterIdentity,
            Self::DestinationRealm => AvpDataType::DiameterIdentity,
            Self::DestinationHost => AvpDataType::DiameterIdentity,
            Self::OriginRealm => AvpDataType::DiameterIdentity,
            Self::SubscriptionData => AvpDataType::Grouped,
            Self::UlrFlags => AvpDataType::Unsigned32,
            Self::UlaFlags => AvpDataType::Unsigned32,
            Self::VisitedPlmnId => AvpDataType::OctetString,
            Self::RequestedEutranAuthInfo => AvpDataType::Grouped,
            Self::ChargingRuleInstall => AvpDataType::Grouped,
            Self::ChargingRuleName => AvpDataType::OctetString,
            Self::EventTrigger => AvpDataType::Enumerated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u32() {
        assert_eq!(StandardAvpCode::from_u32(264), Some(StandardAvpCode::OriginHost));
        assert_eq!(StandardAvpCode::from_u32(268), Some(StandardAvpCode::ResultCode));
        assert_eq!(StandardAvpCode::from_u32(9999), None);
    }

    #[test]
    fn test_name() {
        assert_eq!(StandardAvpCode::OriginHost.name(), "Origin-Host");
        assert_eq!(StandardAvpCode::ResultCode.name(), "Result-Code");
    }

    #[test]
    fn test_data_type() {
        assert_eq!(StandardAvpCode::OriginHost.data_type(), AvpDataType::DiameterIdentity);
        assert_eq!(StandardAvpCode::ResultCode.data_type(), AvpDataType::Unsigned32);
    }
}
