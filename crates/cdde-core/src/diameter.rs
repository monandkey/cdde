use crate::error::{CddeError, Result};

/// Diameter packet header (20 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct DiameterHeader {
    pub version: u8,
    pub length: u32,
    pub flags: u8,
    pub command_code: u32,
    pub application_id: u32,
    pub hop_by_hop_id: u32,
    pub end_to_end_id: u32,
}

/// Diameter AVP
#[derive(Debug, Clone, PartialEq)]
pub struct DiameterAvp {
    pub code: u32,
    pub flags: u8,
    pub vendor_id: Option<u32>,
    pub data: Vec<u8>,
}

/// Complete Diameter packet
#[derive(Debug, Clone)]
pub struct DiameterPacket {
    pub header: DiameterHeader,
    pub avps: Vec<DiameterAvp>,
}

// Header flags
pub const FLAG_REQUEST: u8 = 0x80;
pub const FLAG_PROXIABLE: u8 = 0x40;
pub const FLAG_ERROR: u8 = 0x20;
pub const FLAG_RETRANSMIT: u8 = 0x10;

// AVP flags
pub const AVP_FLAG_VENDOR: u8 = 0x80;
pub const AVP_FLAG_MANDATORY: u8 = 0x40;
pub const AVP_FLAG_PROTECTED: u8 = 0x20;

impl DiameterHeader {
    /// Parse header from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 20 {
            return Err(CddeError::InvalidPacket("Header too short".to_string()));
        }

        let version = data[0];
        if version != 1 {
            return Err(CddeError::InvalidPacket(format!("Invalid version: {version}")));
        }

        let length = u32::from_be_bytes([data[1], data[2], data[3], 0]) >> 8;
        let flags = data[4];
        let command_code = u32::from_be_bytes([data[5], data[6], data[7], 0]) >> 8;
        let application_id = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let hop_by_hop_id = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let end_to_end_id = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);

        Ok(Self {
            version,
            length,
            flags,
            command_code,
            application_id,
            hop_by_hop_id,
            end_to_end_id,
        })
    }

    /// Serialize header to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20);

        bytes.push(self.version);

        let length_bytes = self.length.to_be_bytes();
        bytes.extend_from_slice(&length_bytes[1..4]);

        bytes.push(self.flags);

        let cmd_bytes = self.command_code.to_be_bytes();
        bytes.extend_from_slice(&cmd_bytes[1..4]);

        bytes.extend_from_slice(&self.application_id.to_be_bytes());
        bytes.extend_from_slice(&self.hop_by_hop_id.to_be_bytes());
        bytes.extend_from_slice(&self.end_to_end_id.to_be_bytes());

        bytes
    }

    /// Check if this is a request
    pub fn is_request(&self) -> bool {
        (self.flags & FLAG_REQUEST) != 0
    }

    /// Check if this is an answer
    pub fn is_answer(&self) -> bool {
        !self.is_request()
    }
}

impl DiameterAvp {
    /// Parse AVP from bytes
    pub fn parse(data: &[u8]) -> Result<(Self, usize)> {
        if data.len() < 8 {
            return Err(CddeError::InvalidPacket("AVP too short".to_string()));
        }

        let code = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let flags = data[4];
        let length = u32::from_be_bytes([0, data[5], data[6], data[7]]) as usize;

        if length < 8 {
            return Err(CddeError::InvalidPacket("Invalid AVP length".to_string()));
        }

        let mut offset = 8;
        let vendor_id = if (flags & AVP_FLAG_VENDOR) != 0 {
            if data.len() < 12 {
                return Err(CddeError::InvalidPacket("Vendor AVP too short".to_string()));
            }
            let vid = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
            offset = 12;
            Some(vid)
        } else {
            None
        };

        let data_length = length - offset;
        if data.len() < offset + data_length {
            return Err(CddeError::InvalidPacket("AVP data truncated".to_string()));
        }

        let avp_data = data[offset..offset + data_length].to_vec();

        // Calculate padding (align to 4 bytes)
        let padded_length = length.div_ceil(4) * 4;

        Ok((
            Self {
                code,
                flags,
                vendor_id,
                data: avp_data,
            },
            padded_length,
        ))
    }

    /// Serialize AVP to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.code.to_be_bytes());
        bytes.push(self.flags);

        let data_offset = if self.vendor_id.is_some() { 12 } else { 8 };
        let length = data_offset + self.data.len();
        let length_bytes = (length as u32).to_be_bytes();
        bytes.extend_from_slice(&length_bytes[1..4]);

        if let Some(vid) = self.vendor_id {
            bytes.extend_from_slice(&vid.to_be_bytes());
        }

        bytes.extend_from_slice(&self.data);

        // Add padding
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }

        bytes
    }
}

impl DiameterPacket {
    /// Parse complete packet from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let header = DiameterHeader::parse(data)?;

        if data.len() < header.length as usize {
            return Err(CddeError::InvalidPacket("Packet truncated".to_string()));
        }

        let mut avps = Vec::new();
        let mut offset = 20;

        while offset < header.length as usize {
            let (avp, avp_length) = DiameterAvp::parse(&data[offset..])?;
            avps.push(avp);
            offset += avp_length;
        }

        Ok(Self { header, avps })
    }

    /// Serialize packet to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Serialize AVPs first to calculate total length
        let mut avp_bytes = Vec::new();
        for avp in &self.avps {
            avp_bytes.extend_from_slice(&avp.serialize());
        }

        // Update header length
        let total_length = 20 + avp_bytes.len();
        let mut header = self.header.clone();
        header.length = total_length as u32;

        bytes.extend_from_slice(&header.serialize());
        bytes.extend_from_slice(&avp_bytes);

        bytes
    }

    /// Find AVP by code
    pub fn find_avp(&self, code: u32) -> Option<&DiameterAvp> {
        self.avps.iter().find(|avp| avp.code == code)
    }

    /// Get all AVPs with specific code
    pub fn find_all_avps(&self, code: u32) -> Vec<&DiameterAvp> {
        self.avps.iter().filter(|avp| avp.code == code).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_parse_serialize() {
        let data = vec![
            1, 0, 0, 20, // Version, Length (20)
            0x80, 0, 1, 1, // Flags (Request), Command Code (257)
            0, 0, 0, 0, // Application ID
            0, 0, 0, 1, // Hop-by-Hop ID
            0, 0, 0, 2, // End-to-End ID
        ];

        let header = DiameterHeader::parse(&data).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.length, 20);
        assert_eq!(header.command_code, 257);
        assert!(header.is_request());

        let serialized = header.serialize();
        assert_eq!(serialized, data);
    }

    #[test]
    fn test_avp_parse_serialize() {
        let data = vec![
            0, 0, 1, 8, // Code (264)
            0x40, 0, 0, 12, // Flags (Mandatory), Length (12)
            0x74, 0x65, 0x73, 0x74, // Data "test"
        ];

        let (avp, length) = DiameterAvp::parse(&data).unwrap();
        assert_eq!(avp.code, 264);
        assert_eq!(avp.flags, 0x40);
        assert_eq!(avp.data, b"test");
        assert_eq!(length, 12);

        let serialized = avp.serialize();
        assert_eq!(&serialized[..12], &data[..]);
    }

    #[test]
    fn test_packet_parse() {
        let data = vec![
            1, 0, 0, 32, // Version, Length (32)
            0x80, 0, 1, 1, // Flags, Command Code
            0, 0, 0, 0, // Application ID
            0, 0, 0, 1, // Hop-by-Hop ID
            0, 0, 0, 2, // End-to-End ID
            // AVP
            0, 0, 1, 8, // Code (264)
            0x40, 0, 0, 12, // Flags, Length
            0x74, 0x65, 0x73, 0x74, // Data "test"
        ];

        let packet = DiameterPacket::parse(&data).unwrap();
        assert_eq!(packet.header.command_code, 257);
        assert_eq!(packet.avps.len(), 1);
        assert_eq!(packet.avps[0].code, 264);
    }
}
