use crate::packet::{BGPHeader, BGPMessageType, NotificationMessage, OpenMessage, BGP_HEADER_LEN};

pub enum BGPMessage {
    Open(OpenMessage),
    KeepAlive,
    Notification(NotificationMessage),
}

impl BGPMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let pkt_bytes: Vec<u8>;

        match self {
            BGPMessage::Open(open_msg) => {
                let payload = open_msg.serialize_payload();
                pkt_bytes = payload;
            }
            BGPMessage::KeepAlive => {
                let hdr = BGPHeader::new(BGPMessageType::KeepAlive, BGP_HEADER_LEN as u16);
                pkt_bytes = hdr.serialize()
            }
            BGPMessage::Notification(notification_msg) => {
                let payload = notification_msg.serialize_payload();
                pkt_bytes = payload;
            }
        }

        pkt_bytes
    }
}
