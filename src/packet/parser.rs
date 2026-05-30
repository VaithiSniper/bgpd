use crate::packet::{
    parse_notification_msg, parse_open_msg, BGPHeader, BGPMessage, BGPMessageType, BGP_HEADER_LEN,
};

// | Marker (16 bytes) | Length (2 bytes) | Message Type (1 byte) |
pub fn parse_header(buf: &[u8]) -> Result<BGPHeader, String> {
    if buf.len() < BGP_HEADER_LEN {
        return Err(String::from("Not enough bytes for BGP Header"));
    }

    let marker: [u8; 16] = buf[0..16].try_into().unwrap();
    let length = u16::from_be_bytes([buf[16], buf[17]]);
    let msg_type = BGPMessageType::from_u8(buf[18])?;

    Ok(BGPHeader {
        marker,
        length,
        msg_type,
    })
}

pub fn parse_message(buf: &[u8]) -> Result<BGPMessage, String> {
    let hdr = parse_header(buf)?;
    let payload_buf = &buf[BGP_HEADER_LEN..];

    match hdr.msg_type {
        BGPMessageType::Open => {
            let open = parse_open_msg(payload_buf)?;
            return Ok(BGPMessage::Open(open));
        }
        BGPMessageType::KeepAlive => return Ok(BGPMessage::KeepAlive),
        BGPMessageType::Close => {}
        BGPMessageType::Update => {}
        BGPMessageType::Notification => {
            let notification = parse_notification_msg(payload_buf)?;
            return Ok(BGPMessage::Notification(notification));
        }
    }

    Err("Unknown BGP Message Type received, skipping processing".to_string())
}
