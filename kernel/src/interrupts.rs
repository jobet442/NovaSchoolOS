use crate::sync::Spinlock;

// Simulated Interrupt Descriptor Table
pub struct InterruptDescriptorTable {
    pub timer_handler: fn(),
    pub keyboard_handler: fn(u8), // scancode parameter
    pub page_fault_handler: fn(u64, bool), // (vaddr, write_fault)
}

static mut IDT: Option<InterruptDescriptorTable> = None;

// Counters for visualization
static TIMER_INTERRUPT_COUNT: Spinlock<u64> = Spinlock::new(0);
static KEYBOARD_INTERRUPT_COUNT: Spinlock<u64> = Spinlock::new(0);
static PAGE_FAULT_COUNT: Spinlock<u64> = Spinlock::new(0);

pub fn init_interrupts() {
    unsafe {
        IDT = Some(InterruptDescriptorTable {
            timer_handler: handle_timer_tick,
            keyboard_handler: handle_keyboard_input,
            page_fault_handler: handle_page_fault_ex,
        });
    }

    *TIMER_INTERRUPT_COUNT.lock() = 0;
    *KEYBOARD_INTERRUPT_COUNT.lock() = 0;
    *PAGE_FAULT_COUNT.lock() = 0;
}

fn handle_timer_tick() {
    let mut count = TIMER_INTERRUPT_COUNT.lock();
    *count += 1;

    #[cfg(not(target_os = "none"))]
    super::scheduler::schedule_next();
}

fn handle_keyboard_input(scancode: u8) {
    let mut count = KEYBOARD_INTERRUPT_COUNT.lock();
    *count += 1;

    // Convert scancode to char event and push to driver queue
    // Simple ASCII decoder for demo
    if scancode > 0 {
        let ch = scancode as char;
        drivers::input::push_key_event(drivers::input::KeyEvent {
            code: drivers::input::KeyCode::Char(ch),
            shift: false,
            ctrl: false,
            alt: false,
        });
    }
}

fn handle_page_fault_ex(vaddr: u64, write_fault: bool) {
    let _ = vaddr;
    let _ = write_fault;
    let mut count = PAGE_FAULT_COUNT.lock();
    *count += 1;

    #[cfg(not(target_os = "none"))]
    {
        let current_pid = super::task::get_current_pid();
        if let Err(e) = super::mem::handle_page_fault(current_pid, vaddr, write_fault) {
            drivers::vga_println!("[Exception 14] Page Fault exception unhandled: {}", e);
            // Terminate faulted process
            let _ = super::task::kill_process(current_pid);
        }
    }
}

// Dispatches an interrupt trigger through the IDT
pub fn trigger_interrupt_vector(vector: u8, arg: u64) {
    unsafe {
        if let Some(ref idt) = IDT {
            match vector {
                0x20 => (idt.timer_handler)(), // Vector 32: Timer
                0x21 => (idt.keyboard_handler)(arg as u8), // Vector 33: Keyboard
                0x0E => (idt.page_fault_handler)(arg, true), // Vector 14: Page Fault (assume write)
                _ => {}
            }
        }
    }
}

// Get diagnostic stats for visualizer
pub fn get_interrupt_stats() -> (u64, u64, u64) {
    (
        *TIMER_INTERRUPT_COUNT.lock(),
        *KEYBOARD_INTERRUPT_COUNT.lock(),
        *PAGE_FAULT_COUNT.lock(),
    )
}
