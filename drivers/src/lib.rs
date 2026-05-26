#![cfg_attr(target_os = "none", no_std)]

pub mod mutex;
pub mod vga;
pub mod input;

#[cfg(not(target_os = "none"))]
pub mod disk;

#[cfg(not(target_os = "none"))]
pub mod network;

pub fn init_drivers() {
    vga::init_vga();
    input::init_input();
    
    #[cfg(not(target_os = "none"))]
    disk::init_disk();
    
    #[cfg(not(target_os = "none"))]
    network::init_network();
}
