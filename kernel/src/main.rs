#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;

#[cfg(target_os = "none")]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize drivers (VGA console, keyboard)
    drivers::init_drivers();

    // 2. Initialize kernel components (Memory, Interrupts)
    kernel::init_kernel();

    drivers::vga_println!("=======================================================");
    drivers::vga_println!("         NovaSchool OS Bare-Metal Kernel Booted!       ");
    drivers::vga_println!("=======================================================");
    drivers::vga_println!("System running in raw no_std / target_os = none mode.");
    drivers::vga_println!("Preemptive multitasking and paging initialized.");

    // Loop forever
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
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
    println!("NovaSchool OS bare-metal kernel stub for host. Run the simulator via cargo run.");
}
