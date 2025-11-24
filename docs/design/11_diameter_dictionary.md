## Diameter 辞書管理詳細設計

---

## 1. Diameter 辞書の概要

Diameter辞書は、AVP (Attribute-Value Pair) の定義情報を管理するメタデータです。

### 1.1. 辞書の役割

- **AVPコードの名前解決**: `264` → `Origin-Host`
- **データ型の特定**: `OctetString`, `Unsigned32`, `Grouped` など
- **必須フラグの判定**: Mandatory (M), Vendor-specific (V)
- **ベンダーID管理**: 3GPP (10415), ETSI (13019) など

---

## 2. 辞書の種類

CDDEでは、2種類の辞書を使用します。

| 種別 | 説明 | 形式 | ロード方法 |
|:---|:---|:---|:---|
| **標準辞書** | RFC 6733, 3GPP TS 29.xxx で定義されたAVP | Rustコード (Static) | コンパイル時に埋め込み |
| **ベンダー拡張辞書** | オペレータ独自のAVP定義 | XML/JSON | 起動時に動的ロード |

---

## 3. 標準辞書の実装 (Static Dictionary)

### 3.1. Rust Enum による定義

```rust
// diameter-dict/src/standard.rs

/// 標準AVPコード定義
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum StandardAvpCode {
    // RFC 6733 Base Protocol
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
    OriginRealm = 296,
    DestinationHost = 293,
    DestinationRealm = 283,
    RouteRecord = 282,
    
    // 3GPP S6a (TS 29.272)
    SubscriptionData = 1400,
    UlrFlags = 1405,
    UlaFlags = 1406,
    VisitedPlmnId = 1407,
    RequestedEutranAuthInfo = 1408,
    
    // 3GPP Gx (TS 29.212)
    ChargingRuleInstall = 1001,
    ChargingRuleName = 1005,
    EventTrigger = 1006,
    
    // ... (他のAVPを追加)
}

impl StandardAvpCode {
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::UserName),
            257 => Some(Self::HostIpAddress),
            264 => Some(Self::OriginHost),
            268 => Some(Self::ResultCode),
            // ... (全てのマッピング)
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::UserName => "User-Name",
            Self::OriginHost => "Origin-Host",
            Self::ResultCode => "Result-Code",
            // ...
            _ => "Unknown",
        }
    }

    pub fn data_type(&self) -> AvpDataType {
        match self {
            Self::UserName => AvpDataType::Utf8String,
            Self::OriginHost => AvpDataType::DiameterIdentity,
            Self::ResultCode => AvpDataType::Unsigned32,
            Self::SubscriptionData => AvpDataType::Grouped,
            // ...
            _ => AvpDataType::OctetString,
        }
    }
}
```

### 3.2. データ型定義

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl AvpDataType {
    /// データ型に応じたパース処理
    pub fn parse(&self, data: &[u8]) -> Result<AvpValue, ParseError> {
        match self {
            Self::Unsigned32 => {
                if data.len() != 4 {
                    return Err(ParseError::InvalidLength);
                }
                let value = u32::from_be_bytes(data.try_into().unwrap());
                Ok(AvpValue::Unsigned32(value))
            }
            Self::Utf8String => {
                let s = String::from_utf8(data.to_vec())
                    .map_err(|_| ParseError::InvalidUtf8)?;
                Ok(AvpValue::Utf8String(s))
            }
            Self::Grouped => {
                // Grouped AVPは再帰的にパース
                let avps = parse_grouped_avp(data)?;
                Ok(AvpValue::Grouped(avps))
            }
            // ... (他のデータ型)
            _ => Ok(AvpValue::OctetString(data.to_vec())),
        }
    }
}
```

---

## 4. ベンダー拡張辞書 (Dynamic Dictionary)

### 4.1. XML 辞書フォーマット

```xml
<!-- vendor-dict-example.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<diameter-dictionary>
  <vendor id="99999" name="CustomOperator">
    <avp code="50001" name="Custom-Session-Info" data-type="Grouped" mandatory="true">
      <description>Custom session tracking information</description>
    </avp>
    
    <avp code="50002" name="Custom-User-Category" data-type="Enumerated" mandatory="false">
      <description>User category for billing</description>
      <enum value="1" name="PREMIUM"/>
      <enum value="2" name="STANDARD"/>
      <enum value="3" name="TRIAL"/>
    </avp>
    
    <avp code="50003" name="Custom-Quota-Limit" data-type="Unsigned64" mandatory="true">
      <description>Data quota limit in bytes</description>
    </avp>
  </vendor>
</diameter-dictionary>
```

### 4.2. 動的辞書のロード

```rust
// diameter-dict/src/dynamic.rs

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct DynamicDictionary {
    #[serde(rename = "vendor")]
    vendors: Vec<VendorDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct VendorDefinition {
    #[serde(rename = "@id")]
    id: u32,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "avp")]
    avps: Vec<AvpDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AvpDefinition {
    #[serde(rename = "@code")]
    code: u32,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@data-type")]
    data_type: String,
    #[serde(rename = "@mandatory")]
    mandatory: bool,
    #[serde(rename = "description")]
    description: Option<String>,
}

impl DynamicDictionary {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let xml_content = std::fs::read_to_string(path)?;
        let dict: DynamicDictionary = quick_xml::de::from_str(&xml_content)?;
        Ok(dict)
    }

    pub fn to_lookup_table(&self) -> HashMap<u32, AvpDefinition> {
        let mut table = HashMap::new();
        for vendor in &self.vendors {
            for avp in &vendor.avps {
                table.insert(avp.code, avp.clone());
            }
        }
        table
    }
}
```

---

## 5. 統合辞書マネージャー

### 5.1. Dictionary Manager 実装

```rust
// diameter-dict/src/manager.rs

pub struct DictionaryManager {
    // 標準辞書は常に利用可能
    standard: StandardDictionary,
    // ベンダー拡張辞書 (動的ロード)
    dynamic: HashMap<u32, AvpDefinition>,
}

impl DictionaryManager {
    pub fn new() -> Self {
        Self {
            standard: StandardDictionary::new(),
            dynamic: HashMap::new(),
        }
    }

    /// ベンダー辞書をロード
    pub fn load_vendor_dict(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dict = DynamicDictionary::load_from_file(path)?;
        self.dynamic.extend(dict.to_lookup_table());
        Ok(())
    }

    /// AVPコードから定義を検索
    pub fn lookup(&self, code: u32) -> Option<AvpInfo> {
        // 1. 標準辞書を優先
        if let Some(std_code) = StandardAvpCode::from_u32(code) {
            return Some(AvpInfo {
                code,
                name: std_code.name().to_string(),
                data_type: std_code.data_type(),
                vendor_id: None,
            });
        }

        // 2. ベンダー拡張辞書を検索
        if let Some(def) = self.dynamic.get(&code) {
            return Some(AvpInfo {
                code,
                name: def.name.clone(),
                data_type: parse_data_type(&def.data_type),
                vendor_id: Some(def.code), // ベンダーIDを含む
            });
        }

        None
    }

    /// AVPをパース
    pub fn parse_avp(&self, code: u32, data: &[u8]) -> Result<AvpValue, ParseError> {
        let info = self.lookup(code)
            .ok_or(ParseError::UnknownAvpCode(code))?;
        
        info.data_type.parse(data)
    }
}

#[derive(Debug, Clone)]
pub struct AvpInfo {
    pub code: u32,
    pub name: String,
    pub data_type: AvpDataType,
    pub vendor_id: Option<u32>,
}
```

---

## 6. 辞書の配布と更新

### 6.1. ConfigMap による辞書配布

```yaml
# vendor-dict-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: vendor-dictionary
  namespace: cdde-system
data:
  vendor-dict.xml: |
    <?xml version="1.0" encoding="UTF-8"?>
    <diameter-dictionary>
      <!-- ベンダー辞書の内容 -->
    </diameter-dictionary>
```

### 6.2. DCR Podでの辞書マウント

```yaml
spec:
  containers:
  - name: dcr
    volumeMounts:
    - name: vendor-dict
      mountPath: /etc/cdde/dictionaries
      readOnly: true
  volumes:
  - name: vendor-dict
    configMap:
      name: vendor-dictionary
```

### 6.3. 起動時の辞書ロード

```rust
// dcr/src/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dict_manager = DictionaryManager::new();
    
    // ベンダー辞書をロード
    let dict_path = "/etc/cdde/dictionaries/vendor-dict.xml";
    if std::path::Path::new(dict_path).exists() {
        dict_manager.load_vendor_dict(dict_path)?;
        info!("Loaded vendor dictionary from {}", dict_path);
    }

    // DCRメインロジック起動
    let router = DiameterRouter::new(dict_manager);
    router.start().await?;

    Ok(())
}
```

---

## 7. パフォーマンス最適化

### 7.1. 辞書キャッシュ

```rust
use std::sync::Arc;
use dashmap::DashMap;

pub struct CachedDictionaryManager {
    inner: Arc<DictionaryManager>,
    cache: DashMap<u32, Arc<AvpInfo>>,
}

impl CachedDictionaryManager {
    pub fn lookup(&self, code: u32) -> Option<Arc<AvpInfo>> {
        // キャッシュヒット
        if let Some(info) = self.cache.get(&code) {
            return Some(info.clone());
        }

        // キャッシュミス: 辞書から検索
        if let Some(info) = self.inner.lookup(code) {
            let arc_info = Arc::new(info);
            self.cache.insert(code, arc_info.clone());
            return Some(arc_info);
        }

        None
    }
}
```

---

## 8. 辞書のバリデーション

### 8.1. 起動時チェック

```rust
impl DictionaryManager {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // 1. 必須AVPの存在確認
        let required_avps = [
            StandardAvpCode::SessionId,
            StandardAvpCode::OriginHost,
            StandardAvpCode::OriginRealm,
            StandardAvpCode::ResultCode,
        ];

        for &avp in &required_avps {
            if self.lookup(avp as u32).is_none() {
                return Err(ValidationError::MissingRequiredAvp(avp.name()));
            }
        }

        // 2. ベンダー辞書の重複チェック
        // ...

        Ok(())
    }
}
```
