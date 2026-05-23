pub struct OpenMessage {
    pub version: u8,
    pub asn: u16,
    pub hold_time: u16,
    pub bgp_id: u32,
}

impl OpenMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        // Marker
        buf.extend_from_slice(&[0xff; 16]);

        // Length
        buf.extend_from_slice(&29u16.to_be_bytes());

        // Type
        buf.push(1);

        // Version
        buf.push(self.version);

        // ASN
        buf.extend_from_slice(&self.asn.to_be_bytes());

        // Hold time
        buf.extend_from_slice(&self.hold_time.to_be_bytes());

        // BGP Identifier
        buf.extend_from_slice(&self.bgp_id.to_be_bytes());

        // Optional params len
        buf.push(0);

        buf
    }
}
