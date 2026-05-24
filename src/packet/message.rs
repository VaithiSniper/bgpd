use crate::packet::{BGPHeader, BGPMessageType, OpenMessage, BGP_HEADER_LEN};

pub enum BGPMessage {
    Open(OpenMessage),
    KeepAlive,
}

impl BGPMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut pkt_bytes: Vec<u8>;

        match self {
            BGPMessage::Open(open_msg) => {
                let payload = open_msg.serialize_payload();
                pkt_bytes = payload;
            }
            BGPMessage::KeepAlive => {
                let hdr = BGPHeader {
                    marker: [0xff; 16],
                    length: BGP_HEADER_LEN as u16,
                    msg_type: BGPMessageType::KeepAlive,
                };
                pkt_bytes = hdr.serialize()
            }
        }

        pkt_bytes
    }
}
