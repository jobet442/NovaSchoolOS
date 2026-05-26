use std::net::Ipv4Addr;
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait,
}

#[derive(Debug, Clone)]
pub struct TcpSocket {
    pub local_ip: Ipv4Addr,
    pub local_port: u16,
    pub remote_ip: Option<Ipv4Addr>,
    pub remote_port: Option<u16>,
    pub state: TcpState,
    pub rx_buffer: VecDeque<u8>,
    pub tx_buffer: VecDeque<u8>,
}

static SOCKET_TABLE: Mutex<Option<Vec<TcpSocket>>> = Mutex::new(None);

pub fn init_tcp() {
    *SOCKET_TABLE.lock().unwrap() = Some(Vec::new());
}

// Binds a socket to a local port
pub fn bind_socket(port: u16) -> Result<(), String> {
    let mut table = SOCKET_TABLE.lock().unwrap();
    let vec = table.as_mut().ok_or("Socket table uninitialized")?;
    
    // Check if port already bound
    if vec.iter().any(|s| s.local_port == port && s.state == TcpState::Listen) {
        return Err("Port already in use".to_string());
    }

    vec.push(TcpSocket {
        local_ip: Ipv4Addr::new(127, 0, 0, 1),
        local_port: port,
        remote_ip: None,
        remote_port: None,
        state: TcpState::Listen,
        rx_buffer: VecDeque::new(),
        tx_buffer: VecDeque::new(),
    });
    
    Ok(())
}

// Check if there is data on a bound socket
pub fn receive_data_from_port(port: u16) -> Option<Vec<u8>> {
    let mut table = SOCKET_TABLE.lock().unwrap();
    let vec = table.as_mut().unwrap();
    if let Some(socket) = vec.iter_mut().find(|s| s.local_port == port) {
        if !socket.rx_buffer.is_empty() {
            let data: Vec<u8> = socket.rx_buffer.drain(..).collect();
            return Some(data);
        }
    }
    None
}

// Write data into socket buffer to mock transmissions
pub fn send_data_to_port(port: u16, data: &[u8]) -> Result<(), String> {
    let mut table = SOCKET_TABLE.lock().unwrap();
    let vec = table.as_mut().unwrap();
    if let Some(socket) = vec.iter_mut().find(|s| s.local_port == port) {
        socket.tx_buffer.extend(data.iter());
        // For simulation, transmit right away into loopback/driver
        let packet_payload = data.to_vec();
        let frame_payload = super::ipv4::build_ip_packet(
            Ipv4Addr::new(127, 0, 0, 1),
            socket.remote_ip.unwrap_or(Ipv4Addr::new(127, 0, 0, 1)),
            super::ipv4::PROTO_TCP,
            &packet_payload
        );
        let eth_frame = drivers::network::EthFrame {
            src_mac: drivers::network::MY_MAC,
            dest_mac: [0xFF; 6], // Broadcast
            ethertype: 0x0800,   // IPv4
            payload: frame_payload,
        };
        drivers::network::transmit_frame(eth_frame);
        Ok(())
    } else {
        Err("Socket not found".to_string())
    }
}

// Internal packet router calls this to deliver packets to TCP layer
pub fn route_incoming_tcp(src_ip: Ipv4Addr, src_port: u16, dest_port: u16, data: &[u8]) {
    let mut table = SOCKET_TABLE.lock().unwrap();
    if let Some(ref mut vec) = *table {
        if let Some(socket) = vec.iter_mut().find(|s| s.local_port == dest_port) {
            // If listening, establish connection
            if socket.state == TcpState::Listen {
                socket.state = TcpState::Established;
                socket.remote_ip = Some(src_ip);
                socket.remote_port = Some(src_port);
            }
            socket.rx_buffer.extend(data.iter());
        }
    }
}
