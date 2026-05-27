use std::sync::Mutex;
use std::collections::HashMap;

// Page size config (4KB)
pub const PAGE_SIZE: usize = 4096;
pub const TOTAL_PHYSICAL_FRAMES: usize = 32768; // 128MB RAM / 4KB

// Frame Allocator State
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOwner {
    Free,
    Kernel,
    Process(u32), // PID
    Shared,
    Cow(u32), // PID owning the COW page reference
}

// Memory Page entry representation
#[derive(Debug, Clone)]
pub struct PageTableEntry {
    pub frame_id: u32,
    pub present: bool,
    pub writable: bool,
    pub user_accessible: bool,
}

// Page Table L1 (maps 512 pages, 2MB virtual range)
#[derive(Debug, Clone)]
pub struct PageTable {
    pub entries: Vec<PageTableEntry>,
}

// Represent memory map database for visualization
pub struct MemoryManager {
    pub frames: Vec<FrameOwner>,
    // PID -> Virtual Address -> PageTableEntry map
    pub process_page_tables: HashMap<u32, HashMap<u64, PageTableEntry>>,
    pub cow_references: HashMap<u32, u32>, // Frame ID -> Ref Count
}

static MM: Mutex<Option<MemoryManager>> = Mutex::new(None);

pub fn init_mem() {
    let mut frames = vec![FrameOwner::Free; TOTAL_PHYSICAL_FRAMES];
    
    // Reserve lower 1MB for BIOS / system
    for i in 0..256 {
        frames[i] = FrameOwner::Kernel;
    }
    // Reserve kernel code space (1MB to 16MB)
    for i in 256..4096 {
        frames[i] = FrameOwner::Kernel;
    }

    *MM.lock().unwrap() = Some(MemoryManager {
        frames,
        process_page_tables: HashMap::new(),
        cow_references: HashMap::new(),
    });
}

// Allocate a physical frame for a process
pub fn allocate_frame(pid: u32) -> Result<u32, String> {
    let mut mm_lock = MM.lock().unwrap();
    let mm = mm_lock.as_mut().ok_or("MM uninitialized")?;

    for i in 4096..TOTAL_PHYSICAL_FRAMES {
        if mm.frames[i] == FrameOwner::Free {
            mm.frames[i] = FrameOwner::Process(pid);
            return Ok(i as u32);
        }
    }
    Err("Out of physical memory frames".to_string())
}

// Free physical frame
pub fn free_frame(frame_id: u32) {
    let mut mm_lock = MM.lock().unwrap();
    if let Some(ref mut mm) = *mm_lock {
        let f = frame_id as usize;
        if f < TOTAL_PHYSICAL_FRAMES {
            mm.frames[f] = FrameOwner::Free;
            mm.cow_references.remove(&frame_id);
        }
    }
}

// Free all physical frames allocated to a process and remove its page table
pub fn free_process_address_space(pid: u32) -> Result<(), String> {
    let mut mm_lock = MM.lock().unwrap();
    let mm = mm_lock.as_mut().ok_or("MM uninitialized")?;

    // Remove page table mapping and get the entries
    if let Some(table) = mm.process_page_tables.remove(&pid) {
        for entry in table.values() {
            let frame_id = entry.frame_id;
            let f = frame_id as usize;
            if f < TOTAL_PHYSICAL_FRAMES {
                // If it is a COW frame, we must decrement references or manage it
                let is_cow = mm.cow_references.contains_key(&frame_id);
                if is_cow {
                    if let Some(refs) = mm.cow_references.get_mut(&frame_id) {
                        *refs -= 1;
                        if *refs <= 1 {
                            // If only one owner left, find them and make their page writable again
                            let mut remaining_owner = None;
                            for (&p, p_table) in &mut mm.process_page_tables {
                                for p_entry in p_table.values_mut() {
                                    if p_entry.frame_id == frame_id {
                                        p_entry.writable = true;
                                        remaining_owner = Some(p);
                                    }
                                }
                            }
                            if let Some(owner) = remaining_owner {
                                mm.frames[f] = FrameOwner::Process(owner);
                            }
                            mm.cow_references.remove(&frame_id);
                        }
                    }
                } else {
                    // Normal frame, free it
                    mm.frames[f] = FrameOwner::Free;
                }
            }
        }
    }
    Ok(())
}

// Map a virtual page to a physical frame
pub fn map_page(pid: u32, virtual_addr: u64, frame_id: u32, writable: bool) -> Result<(), String> {
    let mut mm_lock = MM.lock().unwrap();
    let mm = mm_lock.as_mut().unwrap();

    let page_addr = virtual_addr & !(PAGE_SIZE as u64 - 1);
    let entry = PageTableEntry {
        frame_id,
        present: true,
        writable,
        user_accessible: true,
    };

    let table = mm.process_page_tables.entry(pid).or_insert_with(HashMap::new);
    table.insert(page_addr, entry);
    Ok(())
}

// Emulate Copy-On-Write fork
pub fn fork_address_space(parent_pid: u32, child_pid: u32) -> Result<(), String> {
    let mut mm_lock = MM.lock().unwrap();
    let mm = mm_lock.as_mut().unwrap();

    // Copy parent page table mapping to child, but mark all writable pages read-only
    let parent_tables = if let Some(table) = mm.process_page_tables.get(&parent_pid) {
        table.clone()
    } else {
        return Ok(()); // empty
    };

    let mut child_table = HashMap::new();

    for (vaddr, entry) in parent_tables {
        let frame = entry.frame_id;
        
        // Mark both parent and child page table entry as read-only
        let mut cow_entry = entry.clone();
        cow_entry.writable = false;

        // If it was writable, increment COW count
        if entry.writable {
            let refs = mm.cow_references.entry(frame).or_insert(1);
            *refs += 1;
            
            // Update owner labels for visualizations
            mm.frames[frame as usize] = FrameOwner::Cow(parent_pid);
        }

        child_table.insert(vaddr, cow_entry);
    }

    // Apply updated read-only constraints back to parent
    if let Some(parent_table) = mm.process_page_tables.get_mut(&parent_pid) {
        for entry in parent_table.values_mut() {
            if entry.writable {
                entry.writable = false;
            }
        }
    }

    mm.process_page_tables.insert(child_pid, child_table);
    Ok(())
}

// Emulate Page Fault Interrupt Handler (COW resolver)
pub fn handle_page_fault(pid: u32, virtual_addr: u64, write_fault: bool) -> Result<(), String> {
    let mut mm_lock = MM.lock().unwrap();
    let mm = mm_lock.as_mut().unwrap();

    let page_addr = virtual_addr & !(PAGE_SIZE as u64 - 1);
    
    // Find page table entry
    let (frame_id, is_cow) = {
        let table = mm.process_page_tables.get(&pid).ok_or("Process page table not found")?;
        let entry = table.get(&page_addr).ok_or("Segmentation Fault (Address not mapped)")?;
        
        if entry.present && !entry.writable && write_fault {
            // It is read-only page fault
            let refs = mm.cow_references.get(&entry.frame_id).copied().unwrap_or(0);
            (entry.frame_id, refs > 1)
        } else if !entry.present {
            return Err("Segmentation Fault (Page not present)".to_string());
        } else {
            return Ok(()); // Writable page fault resolved easily
        }
    };

    if is_cow {
        // Resolve COW: Copy the page contents onto a new physical frame!
        // 1. Allocate new frame
        let mut new_frame = 0;
        for i in 4096..TOTAL_PHYSICAL_FRAMES {
            if mm.frames[i] == FrameOwner::Free {
                mm.frames[i] = FrameOwner::Process(pid);
                new_frame = i as u32;
                break;
            }
        }
        if new_frame == 0 {
            return Err("Out of physical frames resolving COW".to_string());
        }

        // In a real OS we copy raw memory. Here we simulate it.
        // 1. Update active process page entry to the new frame, making it writable
        let table = mm.process_page_tables.get_mut(&pid).unwrap();
        if let Some(entry) = table.get_mut(&page_addr) {
            entry.frame_id = new_frame;
            entry.writable = true;
        }

        // 2. Decrement COW references count
        if let Some(refs) = mm.cow_references.get_mut(&frame_id) {
            *refs -= 1;
            if *refs <= 1 {
                // If only one owner left, make it writable again and revert from Cow tag
                for (&p, table) in &mut mm.process_page_tables {
                    if let Some(entry) = table.get_mut(&page_addr) {
                        if entry.frame_id == frame_id {
                            entry.writable = true;
                            mm.frames[frame_id as usize] = FrameOwner::Process(p);
                        }
                    }
                }
            }
        }

        drivers::vga_println!("[Kernel Page Fault] Resolved COW for PID {}, mapped vaddr 0x{:X} to new frame {}", pid, virtual_addr, new_frame);
    } else {
        return Err("Segmentation Fault (Attempted write to read-only page)".to_string());
    }

    Ok(())
}

// Get memory tables state snapshot for GUI visualization
pub fn get_memory_snapshot() -> (Vec<FrameOwner>, HashMap<u32, usize>) {
    let mm_lock = MM.lock().unwrap();
    if let Some(ref mm) = *mm_lock {
        // Sum pages mapped per PID
        let mut map = HashMap::new();
        for (&pid, table) in &mm.process_page_tables {
            map.insert(pid, table.len());
        }
        (mm.frames.clone(), map)
    } else {
        (Vec::new(), HashMap::new())
    }
}

pub fn get_frame_mapping(frame_id: u32) -> Option<(u32, u64, bool)> {
    let mm_lock = MM.lock().unwrap();
    if let Some(ref mm) = *mm_lock {
        for (&pid, table) in &mm.process_page_tables {
            for (&vaddr, entry) in table {
                if entry.frame_id == frame_id {
                    return Some((pid, vaddr, entry.writable));
                }
            }
        }
    }
    None
}
