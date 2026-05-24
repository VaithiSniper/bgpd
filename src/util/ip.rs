use std::net::Ipv4Addr;

pub fn ipv4_str_to_u32(ip: &str) -> Result<u32, String> {
    let addr: Ipv4Addr = ip
        .parse()
        .map_err(|e| format!("Not a valid IPv4 address: {}", e))?;
    Ok(u32::from(addr))
}

pub fn u32_to_ipv4_str(buf: u32) -> String {
    Ipv4Addr::from(buf).to_string()
}

pub fn format_bgp_id(id: u32) -> String {
    let ip = u32_to_ipv4_str(id);

    if ip == "0.0.0.0" {
        format!("{}", ip)
    } else {
        ip
    }
}
