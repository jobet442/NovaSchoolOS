use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::collections::VecDeque;

static TERMINAL_LOCKED: AtomicBool = AtomicBool::new(false);
static BROADCASTED_ANNOUNCEMENTS: Mutex<Option<VecDeque<String>>> = Mutex::new(None);

pub fn init_classroom() {
    TERMINAL_LOCKED.store(false, Ordering::SeqCst);
    *BROADCASTED_ANNOUNCEMENTS.lock().unwrap() = Some(VecDeque::new());
}

pub fn is_terminal_locked() -> bool {
    TERMINAL_LOCKED.load(Ordering::SeqCst)
}

pub fn set_terminal_lock(locked: bool) {
    TERMINAL_LOCKED.store(locked, Ordering::SeqCst);
}

pub fn broadcast_announcement(msg: &str) {
    let mut lock = BROADCASTED_ANNOUNCEMENTS.lock().unwrap();
    if let Some(ref mut queue) = *lock {
        if queue.len() > 10 {
            queue.pop_front();
        }
        queue.push_back(msg.to_string());
    }
    
    // Also print directly into VGA buffer for instant student alert
    drivers::vga_println!("\n*** CLASSROOM ANNOUNCEMENT: {} ***", msg);
}

pub fn get_announcements() -> Vec<String> {
    let lock = BROADCASTED_ANNOUNCEMENTS.lock().unwrap();
    if let Some(ref queue) = *lock {
        queue.iter().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn distribute_lab_assignment() -> Result<(), String> {
    // Distributes assignment file to student1 workspace
    let fd = filesystem::vfs_open("/students/student1001/lab1_handout.txt", true, true, 0)?;
    let content = "NOVA_LAB_ASSIGNMENT_1\nInstructions: Write a simple script to verify paging COW behavior.\nStatus: IN_PROGRESS\n";
    filesystem::vfs_write(fd, content.as_bytes(), 0)?;
    filesystem::vfs_close(fd)?;
    
    broadcast_announcement("Lab assignment 1 handout distributed to workspaces.");
    Ok(())
}
