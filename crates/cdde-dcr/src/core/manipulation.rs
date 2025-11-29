use cdde_shared::{DiameterMessage, Avp};
use bytes::Bytes;
use regex::Regex;

// DSLで定義されるルールの内部表現
#[derive(Debug)]
pub enum ManipulationRule {
    // AVPの値を置換 (例: Origin-Hostを書き換え)
    ReplaceAvp { code: u32, new_value: Bytes },
    // 正規表現による置換 (Topology Hiding用)
    RegexReplace { code: u32, pattern: Regex, replacement: String },
    // AVP削除
    RemoveAvp { code: u32 },
}

pub struct ManipulationEngine {
    rules: Vec<ManipulationRule>,
}

impl ManipulationEngine {
    pub fn new(rules: Vec<ManipulationRule>) -> Self {
        Self { rules }
    }

    // 純粋関数: メッセージを受け取り、加工して返す
    pub fn apply(&self, mut msg: DiameterMessage) -> DiameterMessage {
        for rule in &self.rules {
            match rule {
                ManipulationRule::ReplaceAvp { code, new_value } => {
                    // 簡易実装: フラグなどは適当
                    let new_avp = Avp {
                        code: *code,
                        flags: 0x40, 
                        length: (new_value.len() + 8) as u32,
                        vendor_id: None,
                        data: new_value.clone(),
                    };
                    msg.set_avp(new_avp);
                },
                ManipulationRule::RegexReplace { code, pattern, replacement } => {
                    if let Some(avp) = msg.get_avp(*code) {
                        let original_str = avp.as_string();
                        let new_str = pattern.replace(&original_str, replacement.as_str());
                        // 文字列からBytesへ再変換してセット
                        let new_bytes = Bytes::from(new_str.into_owned());
                         let new_avp = Avp {
                            code: *code,
                            flags: avp.flags,
                            length: (new_bytes.len() + 8) as u32,
                            vendor_id: avp.vendor_id,
                            data: new_bytes,
                        };
                        msg.set_avp(new_avp);
                    }
                },
                ManipulationRule::RemoveAvp { code } => {
                    msg.avps.retain(|a| a.code != *code);
                }
            }
        }
        msg
    }
}
