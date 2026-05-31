use crate::packet::{BGPHeader, BGPMessageType};
use crate::util::format_bgp_id;
use std::fmt;

const OPEN_MESSAGE_MIN_LEN: usize = 10;

pub struct OpenMessage {
    pub version: u8,
    pub asn: u16,
    pub hold_time: u16,
    pub bgp_id: u32,
    pub opt_len: u8,
    pub opts: Vec<u8>,
}

impl fmt::Debug for OpenMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OpenMessage {{ \
               version: {}, \
               asn: {}, \
               hold_time: {}, \
               bgp_id: {}, \
               opt_len: {} \
             }}",
            self.version,
            self.asn,
            self.hold_time,
            format_bgp_id(self.bgp_id),
            self.opt_len
        )
    }
}

impl OpenMessage {
    pub fn serialize_payload(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        // Construct header
        let hdr = BGPHeader::new(BGPMessageType::Open, 29u16);
        let hdr_bytes = hdr.serialize();
        buf.extend_from_slice(&hdr_bytes);

        // Version
        buf.push(self.version);
        // ASN
        buf.extend_from_slice(&self.asn.to_be_bytes());
        // Hold time
        buf.extend_from_slice(&self.hold_time.to_be_bytes());
        // BGP Identifier
        buf.extend_from_slice(&self.bgp_id.to_be_bytes());
        // Optional params len
        buf.push(self.opt_len);

        buf
    }
}

pub fn parse_open_msg(buf: &[u8]) -> Result<OpenMessage, String> {
    if buf.len() < OPEN_MESSAGE_MIN_LEN {
        return Err("not enough bytes for BGP Open message".to_string());
    }

    let version = buf[0];
    let asn = u16::from_be_bytes([buf[1], buf[2]]);
    let hold_time = u16::from_be_bytes([buf[3], buf[4]]);
    let bgp_id = u32::from_be_bytes([buf[5], buf[6], buf[7], buf[8]]);
    let opt_len = buf[9];

    if opt_len as usize > 0 {
        println!(
            "Got extra opts for BGP Open message, of len: {}",
            opt_len as usize
        );
    }

    Ok(OpenMessage {
        version,
        asn,
        hold_time,
        bgp_id,
        opt_len,
        opts: vec![0; opt_len as usize],
    })
}
