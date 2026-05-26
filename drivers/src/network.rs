use std::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct EthFrame {
    pub src_mac: [u8; 6],
    pub dest_mac: [u8; 6],
    pub ethertype: u16,
    pub payload: Vec<u8>,
}

static RX_QUEUE: Mutex<VecDeque<EthFrame>> = Mutex::new(VecDeque::new());
static TX_QUEUE: Mutex<VecDeque<EthFrame>> = Mutex::new(VecDeque::new());

// MAC address of the virtual network card
pub const MY_MAC: [u8; 6] = [0x00, 0x16, 0x3e, 0x11, 0x22, 0x33];

pub fn init_network() {
    RX_QUEUE.lock().unwrap().clear();
    TX_QUEUE.lock().unwrap().clear();
}

// Read incoming frame from NIC (non-blocking)
pub fn receive_frame() -> Option<EthFrame> {
    RX_QUEUE.lock().unwrap().pop_front()
}

// Write outgoing frame to NIC
pub fn transmit_frame(frame: EthFrame) {
    // Save to transmit log so visualizer can show it
    TX_QUEUE.lock().unwrap().push_back(frame.clone());
    
    // Simulate networking: If it is loopback or broadcast, loop it back immediately!
    if frame.dest_mac == MY_MAC || frame.dest_mac == [0xFF; 6] {
        let rx_frame = EthFrame {
            src_mac: frame.src_mac,
            dest_mac: frame.dest_mac,
            ethertype: frame.ethertype,
            payload: frame.payload,
        };
        RX_QUEUE.lock().unwrap().push_back(rx_frame);
    }
}

// Simulates an external host sending a packet to us (used by network stack simulator)
pub fn simulate_incoming_packet(src_mac: [u8; 6], ethertype: u16, payload: Vec<u8>) {
    let frame = EthFrame {
        src_mac,
        dest_mac: MY_MAC,
        ethertype,
        payload,
    };
    RX_QUEUE.lock().unwrap().push_back(frame);
}

// Read recent TX logs for visualizations
pub fn pop_tx_log() -> Option<EthFrame> {
    TX_QUEUE.lock().unwrap().pop_front()
}
