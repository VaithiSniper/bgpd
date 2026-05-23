const BGP_HEADER_LEN: usize = 19;

#[derive(Debug)]
pub enum BGPMessageType {
    Open,
    Update,
    Notification,
    KeepAlive,
    Close,
}
impl BGPMessageType {
    pub fn from_u8(byte: u8) -> Result<BGPMessageType, String> {
        match byte {
            1 => Ok(BGPMessageType::Open),
            2 => Ok(BGPMessageType::Update),
            3 => Ok(BGPMessageType::Notification),
            4 => Ok(BGPMessageType::KeepAlive),
            5 => Ok(BGPMessageType::Close),
            _ => Err(format!("Unknown message type {}", byte)),
        }
    }
}

#[derive(Debug)]
pub struct BGPHeader {
    pub marker: [u8; 16],
    pub length: u16,
    pub msg_type: BGPMessageType,
}

impl BGPHeader {
    pub fn serialize(hdr: &BGPHeader) -> Vec<u8> {
        let mut hdr_bytes: Vec<u8> = Vec::new();

        // Hardcoded header values
        hdr_bytes.extend_from_slice(&[0xff; 16]);
        hdr_bytes.extend_from_slice(&29u16.to_be_bytes());
        hdr_bytes.push(1);

        hdr_bytes
    }
}

pub fn parse_header(buf: &[u8]) -> Result<BGPHeader, String> {
    // Validate
    if buf.len() < BGP_HEADER_LEN {
        return Err(String::from("Not enough bytes for BGPHeader"));
    }

    // Extract fields as per RFC
    // | Marker (16 bytes) | Length (2 bytes) | Message Type (1 byte) |
    let marker: [u8; 16] = buf[0..16].try_into().unwrap();
    let length = u16::from_be_bytes([buf[16], buf[17]]);

    // Eventually stuff it into hdr
    let msg_type = BGPMessageType::from_u8(buf[18])?;

    Ok(BGPHeader {
        marker,
        length,
        msg_type,
    })
}
