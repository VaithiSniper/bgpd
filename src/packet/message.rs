use crate::packet::OpenMessage;

pub enum BGPMessage {
    Open(OpenMessage),
}

impl BGPMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut pkt_bytes: Vec<u8>;

        match self {
            BGPMessage::Open(open_msg) => {
                let payload = open_msg.serialize_payload();
                pkt_bytes = payload;
            }
        }

        pkt_bytes
    }
}
