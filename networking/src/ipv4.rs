use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct Ipv4Header {
    pub version: u8,
    pub ttl: u8,
    pub protocol: u8,
    pub src_ip: Ipv4Addr,
    pub dest_ip: Ipv4Addr,
}

#[derive(Debug, Clone)]
pub struct IpPacket {
    pub header: Ipv4Header,
    pub payload: Vec<u8>,
}

pub const PROTO_ICMP: u8 = 1;
pub const PROTO_TCP: u8 = 6;
pub const PROTO_UDP: u8 = 17;

pub fn parse_ip_packet(data: &[u8]) -> Result<IpPacket, String> {
    if data.len() < 20 {
        return Err("IP Packet too short".to_string());
    }
    let version = data[0] >> 4;
    let ttl = data[8];
    let protocol = data[9];
    let src_ip = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
    let dest_ip = Ipv4Addr::new(data[16], data[17], data[18], data[19]);

    let header = Ipv4Header {
        version,
        ttl,
        protocol,
        src_ip,
        dest_ip,
    };

    Ok(IpPacket {
        header,
        payload: data[20..].to_vec(),
    })
}

pub fn build_ip_packet(src: Ipv4Addr, dest: Ipv4Addr, protocol: u8, payload: &[u8]) -> Vec<u8> {
    let mut header = vec![0u8; 20];
    header[0] = 0x45; // Version 4, IHL 5 (20 bytes)
    header[8] = 64;   // TTL
    header[9] = protocol;
    header[12..16].copy_from_slice(&src.octets());
    header[16..20].copy_from_slice(&dest.octets());

    let mut packet = header;
    packet.extend_from_slice(payload);
    packet
}
