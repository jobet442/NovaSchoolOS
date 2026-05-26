pub mod ipv4;
pub mod tcp;
pub mod ssh;

pub fn init_networking() {
    tcp::init_tcp();
    ssh::init_ssh();
}

// Poll the network driver for new frames and push them up the stack
pub fn poll_network_card() {
    if let Some(frame) = drivers::network::receive_frame() {
        // Ethernet layer check: IPv4 protocol ID is 0x0800
        if frame.ethertype == 0x0800 {
            if let Ok(ip_packet) = ipv4::parse_ip_packet(&frame.payload) {
                // If TCP packet
                if ip_packet.header.protocol == ipv4::PROTO_TCP {
                    let payload = ip_packet.payload;
                    if payload.len() >= 4 {
                        // Extract TCP ports
                        let src_port = u16::from_be_bytes([payload[0], payload[1]]);
                        let dest_port = u16::from_be_bytes([payload[2], payload[3]]);
                        let data = &payload[4..]; // simplified TCP header parser
                        
                        tcp::route_incoming_tcp(
                            ip_packet.header.src_ip,
                            src_port,
                            dest_port,
                            data
                        );
                    }
                }
            }
        }
    }

    // Run SSH daemon loop
    ssh::poll_ssh_events();
}
