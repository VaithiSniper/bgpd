pub const BGP_HEADER_LEN: usize = 19;

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

    pub fn to_u8(&self) -> u8 {
        match self {
            BGPMessageType::Open => 1,
            BGPMessageType::Update => 2,
            BGPMessageType::Notification => 3,
            BGPMessageType::KeepAlive => 4,
            BGPMessageType::Close => 5,
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
    pub fn new(msg_type: BGPMessageType, payload_len: u16) -> BGPHeader {
        BGPHeader {
            marker: [0xff; 16],
            length: BGP_HEADER_LEN as u16 + payload_len,
            msg_type,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut hdr_bytes: Vec<u8> = Vec::with_capacity(BGP_HEADER_LEN);
        hdr_bytes.extend_from_slice(&self.marker);
        hdr_bytes.extend_from_slice(&self.length.to_be_bytes());
        hdr_bytes.push(self.msg_type.to_u8());

        hdr_bytes
    }
}
