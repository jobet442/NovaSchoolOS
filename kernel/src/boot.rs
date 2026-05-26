#[cfg(not(target_os = "none"))]
use std::panic::PanicHookInfo;

#[derive(Debug, Clone, Copy)]
pub enum MemoryRegionType {
    Usable,
    Reserved,
    UefiCode,
    UefiData,
    AcpiReclaimable,
    FrameBuffer,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    pub start_address: u64,
    pub length: u64,
    pub region_type: MemoryRegionType,
}

pub static PHYSICAL_MEM_MAP: [MemoryMapEntry; 8] = [
    MemoryMapEntry { start_address: 0x000000, length: 0x0A0000, region_type: MemoryRegionType::Usable },
    MemoryMapEntry { start_address: 0x0A0000, length: 0x060000, region_type: MemoryRegionType::Reserved }, // VGA / Bios ROM
    MemoryMapEntry { start_address: 0x100000, length: 0x7E00000, region_type: MemoryRegionType::Usable }, // 126MB Ram
    MemoryMapEntry { start_address: 0x7F00000, length: 0x100000, region_type: MemoryRegionType::UefiCode },
    MemoryMapEntry { start_address: 0x8000000, length: 0x200000, region_type: MemoryRegionType::UefiData },
    MemoryMapEntry { start_address: 0x8200000, length: 0x100000, region_type: MemoryRegionType::AcpiReclaimable },
    MemoryMapEntry { start_address: 0xFD000000, length: 0x1000000, region_type: MemoryRegionType::FrameBuffer }, // 16MB Video Ram
    MemoryMapEntry { start_address: 0xFE000000, length: 0x2000000, region_type: MemoryRegionType::Reserved },
];

pub fn print_boot_banner() {
    drivers::vga_println!("=======================================================");
    drivers::vga_println!("          _   _                 ____       _               _ ");
    drivers::vga_println!("         | \\ | |               / ___|     | |             | |");
    drivers::vga_println!("         |  \\| | _____   ____  \\___ \\  ___| |__   ___   __| |");
    drivers::vga_println!("         | . ` |/ _ \\ \\ / / _` |___) |/ __| '_ \\ / _ \\ / _` |");
    drivers::vga_println!("         | |\\  | (_) \\ V / (_| |____/| (__| | | | (_) | (_| |");
    drivers::vga_println!("         |_| \\_|\\___/ \\_/ \\__,_|____/ \\___|_| |_|\\___/ \\__,_|");
    drivers::vga_println!("=======================================================");
    drivers::vga_println!("                  NovaSchool OS v0.1.0 (x86_64)");
    drivers::vga_println!("       Initializing hybrid kernel components securely...");
}

pub fn print_uefi_memory_map() {
    drivers::vga_println!("[UEFI Bootloader] Physical Memory Map parsed:");
    for (i, entry) in PHYSICAL_MEM_MAP.iter().enumerate() {
        drivers::vga_println!(
            "  [{}] 0x{:08X} - 0x{:08X} | {:?} ({} KB)",
            i,
            entry.start_address,
            entry.start_address + entry.length - 1,
            entry.region_type,
            entry.length / 1024
        );
    }
}

// Educational Kernel Panic Screen
#[cfg(not(target_os = "none"))]
pub fn trigger_kernel_panic(info: &PanicHookInfo) {
    drivers::vga::init_vga(); // Clear screens
    drivers::vga_println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    drivers::vga_println!("                          KERNEL PANIC DETECTED (System Halted)               ");
    drivers::vga_println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        drivers::vga_println!("Details: {}", s);
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        drivers::vga_println!("Details: {}", s);
    } else {
        drivers::vga_println!("Details: Unknown error occurred.");
    }
    if let Some(location) = info.location() {
        drivers::vga_println!("Location: file '{}', line {}", location.file(), location.line());
    }
    drivers::vga_println!("------------------------------------------------------------------------------");
    drivers::vga_println!("[Diagnostic registers dump]");
    drivers::vga_println!("CR0: 0x80010033    CR2: 0x00007FF800    CR3: 0x0000000000101000    CR4: 0x00000000000006F8");
    drivers::vga_println!("RIP: 0x000000000020108A   RSP: 0x0000000000400F30   RFLAGS: 0x0000000000010202");
    drivers::vga_println!("RAX: 0x0000000000000000   RBX: 0x00000000002030E0   RCX: 0x0000000000000045");
    drivers::vga_println!("------------------------------------------------------------------------------");
    drivers::vga_println!("This crash screen is part of NovaSchool OS interactive labs. Students should");
    drivers::vga_println!("analyze CR2 (Page Fault Linear Address) and RIP to trace memory safety issues.");
}
