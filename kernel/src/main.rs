#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;

#[cfg(target_os = "none")]
static mut SHELL_BUFFER: [u8; 128] = [0; 128];
#[cfg(target_os = "none")]
static mut SHELL_LEN: usize = 0;

#[cfg(target_os = "none")]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

#[cfg(target_os = "none")]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

#[cfg(target_os = "none")]
fn scancode_to_ascii(scancode: u8) -> Option<char> {
    match scancode {
        0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'), 0x05 => Some('4'),
        0x06 => Some('5'), 0x07 => Some('6'), 0x08 => Some('7'), 0x09 => Some('8'),
        0x0A => Some('9'), 0x0B => Some('0'), 0x0C => Some('-'), 0x0D => Some('='),
        0x10 => Some('q'), 0x11 => Some('w'), 0x12 => Some('e'), 0x13 => Some('r'),
        0x14 => Some('t'), 0x15 => Some('y'), 0x16 => Some('u'), 0x17 => Some('i'),
        0x18 => Some('o'), 0x19 => Some('p'), 0x1E => Some('a'), 0x1F => Some('s'),
        0x20 => Some('d'), 0x21 => Some('f'), 0x22 => Some('g'), 0x23 => Some('h'),
        0x24 => Some('j'), 0x25 => Some('k'), 0x26 => Some('l'), 0x2C => Some('z'),
        0x2D => Some('x'), 0x2E => Some('c'), 0x2F => Some('v'), 0x30 => Some('b'),
        0x31 => Some('n'), 0x32 => Some('m'), 0x39 => Some(' '),
        _ => None,
    }
}

#[cfg(target_os = "none")]
unsafe fn execute_bare_metal_command() {
    let cmd = core::str::from_utf8_unchecked(&SHELL_BUFFER[..SHELL_LEN]);
    let cmd_trimmed = cmd.trim();
    
    if cmd_trimmed == "help" {
        drivers::vga_println!("Bare-Metal Commands:");
        drivers::vga_println!("  help     - Show this list");
        drivers::vga_println!("  clear    - Clear display");
        drivers::vga_println!("  panic    - Trigger a kernel exception panic");
        drivers::vga_println!("  reboot   - Reboot target CPU");
    } else if cmd_trimmed == "clear" {
        if let Some(ref mut writer) = *drivers::vga::VGA_WRITER.lock() {
            writer.clear_screen();
            
            let ptr = 0xb8000 as *mut u8;
            for i in 0..drivers::vga::VGA_BUFFER_SIZE {
                core::ptr::write_volatile(ptr.add(i), 0);
            }
        }
    } else if cmd_trimmed == "panic" {
        panic!("Student-initiated bare metal panic exception test!");
    } else if cmd_trimmed == "reboot" {
        drivers::vga_println!("Resetting CPU...");
        outb(0x64, 0xFE);
    } else if !cmd_trimmed.is_empty() {
        drivers::vga_println!("Unknown bare-metal command: {}", cmd_trimmed);
    }
    
    SHELL_LEN = 0;
}

#[cfg(target_os = "none")]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize drivers (VGA console, keyboard)
    drivers::init_drivers();

    // 2. Initialize kernel components (Memory, Interrupts)
    kernel::init_kernel();

    drivers::vga_println!("=======================================================");
    drivers::vga_println!("         NovaOS Bare-Metal Kernel Booted!       ");
    drivers::vga_println!("=======================================================");
    drivers::vga_println!("System running in raw no_std / target_os = none mode.");
    drivers::vga_println!("Preemptive multitasking and paging initialized.");
    drivers::vga_println!();
    drivers::vga_print!("bare-metal-os# ");

    loop {
        unsafe {
            if (inb(0x64) & 1) != 0 {
                let scancode = inb(0x60);
                if (scancode & 0x80) == 0 { // key down
                    if scancode == 0x1C { // Enter
                        drivers::vga_println!();
                        execute_bare_metal_command();
                        drivers::vga_print!("bare-metal-os# ");
                    } else if scancode == 0x0E { // Backspace
                        if SHELL_LEN > 0 {
                            SHELL_LEN -= 1;
                            let mut cursor_lock = drivers::vga::VGA_CURSOR.lock();
                            let (row, col) = *cursor_lock;
                            if col > 0 {
                                *cursor_lock = (row, col - 1);
                                let offset = (row * drivers::vga::VGA_WIDTH + (col - 1)) * 2;
                                let mut vga_buf = drivers::vga::VGA_BUFFER.lock();
                                vga_buf[offset] = b' ';
                                
                                let ptr = (0xb8000 + offset) as *mut u8;
                                core::ptr::write_volatile(ptr, b' ');
                            }
                        }
                    } else if let Some(ch) = scancode_to_ascii(scancode) {
                        if SHELL_LEN < 120 {
                            SHELL_BUFFER[SHELL_LEN] = ch as u8;
                            SHELL_LEN += 1;
                            drivers::vga_print!("{}", ch);
                        }
                    }
                }
            }
            
            #[cfg(target_arch = "x86_64")]
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    drivers::vga_println!("\n!!! BARE-METAL KERNEL PANIC !!!");
    drivers::vga_println!("{}", info);
    loop {}
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("NovaOS bare-metal kernel stub for host. Run the simulator via cargo run.");
}
