#![cfg_attr(target_os = "none", no_std)]

pub mod sync;
pub mod boot;

#[cfg(not(target_os = "none"))]
pub mod mem;

#[cfg(not(target_os = "none"))]
pub mod task;

#[cfg(not(target_os = "none"))]
pub mod scheduler;

pub mod interrupts;

#[cfg(not(target_os = "none"))]
pub mod syscall;

#[cfg(not(target_os = "none"))]
pub mod security;

pub fn init_kernel() {
    #[cfg(not(target_os = "none"))]
    {
        // 1. Initialize Memory Manager
        mem::init_mem();
    }

    // 2. Initialize Interrupt Service Tables
    interrupts::init_interrupts();

    #[cfg(not(target_os = "none"))]
    {
        // 3. Initialize Security Audits
        security::init_security();

        // 4. Initialize Task manager
        task::init_task();

        // 5. Initialize Syscall Tracers
        syscall::init_syscall();
    }

    drivers::vga_println!("[Kernel init] All virtual subsystems configured successfully.");
}
