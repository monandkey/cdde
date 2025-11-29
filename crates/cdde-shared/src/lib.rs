use bytes::Bytes;

// Diameter Command Codes
pub const CMD_CER: u32 = 257;
pub const CMD_DWR: u32 = 280;
pub const CMD_ACR: u32 = 271;

// AVP Codes
pub const AVP_ORIGIN_HOST: u32 = 264;
pub const AVP_ORIGIN_REALM: u32 = 296;
pub const AVP_DEST_REALM: u32 = 283;
pub const AVP_ROUTE_RECORD: u32 = 282;

// Result-Code values (AVP 268)
pub const RESULT_CODE_SUCCESS: u32 = 2001; // DIAMETER_SUCCESS
pub const RESULT_CODE_UNABLE_TO_DELIVER: u32 = 3002; // DIAMETER_UNABLE_TO_DELIVER
pub const AVP_RESULT_CODE: u32 = 268;


#[derive(Debug, Clone, PartialEq)]
pub struct Avp {
    pub code: u32,
    pub flags: u8,
    pub length: u32,
    pub vendor_id: Option<u32>,
    pub data: Bytes, // Zero-copy friendly
}

impl Avp {
    pub fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiameterMessage {
    pub version: u8,
    pub flags: u8,
    pub command_code: u32,
    pub application_id: u32,
    pub hop_by_hop_id: u32,
    pub end_to_end_id: u32,
    pub is_request: bool,
    pub avps: Vec<Avp>,
}

impl DiameterMessage {
    pub fn new(command_code: u32, is_request: bool) -> Self {
        Self {
            version: 1,
            flags: if is_request { 0x80 } else { 0x00 },
            command_code,
            application_id: 0,
            hop_by_hop_id: 0,
            end_to_end_id: 0,
            is_request,
            avps: Vec::new(),
        }
    }

    // Helper: Get specific AVP
    pub fn get_avp(&self, code: u32) -> Option<&Avp> {
        self.avps.iter().find(|a| a.code == code)
    }

    // Helper: Add or replace AVP
    pub fn set_avp(&mut self, avp: Avp) {
        if let Some(existing) = self.avps.iter_mut().find(|a| a.code == avp.code) {
            *existing = avp;
        } else {
            self.avps.push(avp);
        }
    }
    
    // Helper: Check message type
    pub fn is_cer(&self) -> bool { self.command_code == CMD_CER && self.is_request }
    pub fn is_cea(&self) -> bool { self.command_code == CMD_CER && !self.is_request }
    pub fn is_dwr(&self) -> bool { self.command_code == CMD_DWR && self.is_request }
    pub fn is_dwa(&self) -> bool { self.command_code == CMD_DWR && !self.is_request }
}
