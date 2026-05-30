use crate::packet::{BGPHeader, BGPMessageType};
use std::fmt;

const NOTIFICATION_MESSAGE_MIN_LEN: usize = 2;

#[derive(Debug)]
pub enum NotificationErrorCode {
    MessageHeaderError = 1,
    OpenMessageError = 2,
    HoldTimerExpired = 4,
    FSMError = 5,
    Cease = 6,
}
impl NotificationErrorCode {
    pub fn from_u8(byte: u8) -> Result<NotificationErrorCode, String> {
        match byte {
            1 => Ok(NotificationErrorCode::MessageHeaderError),
            2 => Ok(NotificationErrorCode::OpenMessageError),
            4 => Ok(NotificationErrorCode::HoldTimerExpired),
            5 => Ok(NotificationErrorCode::FSMError),
            6 => Ok(NotificationErrorCode::Cease),
            _ => Err(format!("Unknown bgp notification error code {}", byte)),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            NotificationErrorCode::MessageHeaderError => 1,
            NotificationErrorCode::OpenMessageError => 2,
            NotificationErrorCode::HoldTimerExpired => 4,
            NotificationErrorCode::FSMError => 5,
            NotificationErrorCode::Cease => 6,
        }
    }
}

pub struct NotificationMessage {
    pub err_code: NotificationErrorCode,
    pub err_sub_code: u8,
    pub data: Vec<u8>,
}

impl fmt::Debug for NotificationMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NotificationMessage {{ \
               err_code: {:?}, \
               err_sub_code: {}, \
               data: {:?} \
             }}",
            self.err_code, self.err_sub_code, self.data
        )
    }
}

impl NotificationMessage {
    pub fn new(
        err_code: NotificationErrorCode,
        err_sub_code: u8,
        data: Vec<u8>,
    ) -> NotificationMessage {
        NotificationMessage {
            err_code,
            err_sub_code,
            data,
        }
    }
    pub fn serialize_payload(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        // Construct header
        let hdr = BGPHeader {
            marker: [0xff; 16],
            length: 29u16,
            msg_type: BGPMessageType::Notification,
        };
        let hdr_bytes = hdr.serialize();
        buf.extend_from_slice(&hdr_bytes);

        // Error code
        buf.push(self.err_code.to_u8());
        // Error sub code
        buf.push(self.err_sub_code);
        // Data
        buf.extend_from_slice(&self.data.as_slice());

        buf
    }
}

pub fn parse_notification_msg(buf: &[u8]) -> Result<NotificationMessage, String> {
    if buf.len() < NOTIFICATION_MESSAGE_MIN_LEN {
        return Err("not enough bytes for BGP Notification message".to_string());
    }

    let err_code = NotificationErrorCode::from_u8(buf[0])?;
    let err_sub_code = buf[1];
    let mut data: Vec<u8> = Vec::new();
    if (buf.len() - NOTIFICATION_MESSAGE_MIN_LEN) > 0 {
        let data_buf = buf[2..].to_vec();
        println!(
            "Got extra data for BGP Notification message, of len: {}",
            data_buf.len()
        );
        data = buf[2..].to_vec();
    }

    Ok(NotificationMessage {
        err_code,
        err_sub_code,
        data,
    })
}
