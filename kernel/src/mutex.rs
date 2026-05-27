use std::collections::HashMap;
use std::sync::Mutex;
use super::task::{set_process_state, ProcessState, kill_process};

#[derive(Debug, Clone)]
pub struct MutexInfo {
    pub id: u32,
    pub name: String,
    pub owner: Option<u32>, // PID holding the lock
    pub waiters: Vec<u32>, // PIDs blocked waiting
}

static MUTEX_TABLE: Mutex<Option<HashMap<u32, MutexInfo>>> = Mutex::new(None);
static NEXT_MUTEX_ID: Mutex<u32> = Mutex::new(1);

pub fn init_mutex() {
    *MUTEX_TABLE.lock().unwrap() = Some(HashMap::new());
    *NEXT_MUTEX_ID.lock().unwrap() = 1;
}

pub fn create_mutex(name: &str) -> Result<u32, String> {
    let mut table_lock = MUTEX_TABLE.lock().unwrap();
    let table = table_lock.as_mut().ok_or("Mutex table uninitialized")?;
    
    let mut id_lock = NEXT_MUTEX_ID.lock().unwrap();
    let id = *id_lock;
    
    table.insert(id, MutexInfo {
        id,
        name: name.to_string(),
        owner: None,
        waiters: Vec::new(),
    });
    
    *id_lock += 1;
    Ok(id)
}

pub fn lock_mutex(mutex_id: u32, pid: u32) -> Result<bool, String> {
    let mut table_lock = MUTEX_TABLE.lock().unwrap();
    let table = table_lock.as_mut().ok_or("Mutex table uninitialized")?;
    let mutex = table.get_mut(&mutex_id).ok_or("Mutex not found")?;

    if let Some(owner) = mutex.owner {
        if owner == pid {
            return Err("Deadlock: Process already holds this lock".to_string());
        }
        // Add to waiters and block process
        if !mutex.waiters.contains(&pid) {
            mutex.waiters.push(pid);
        }
        set_process_state(pid, ProcessState::Blocked);
        Ok(false) // Not acquired (blocked)
    } else {
        mutex.owner = Some(pid);
        Ok(true) // Acquired
    }
}

pub fn unlock_mutex(mutex_id: u32, pid: u32) -> Result<(), String> {
    let mut table_lock = MUTEX_TABLE.lock().unwrap();
    let table = table_lock.as_mut().ok_or("Mutex table uninitialized")?;
    let mutex = table.get_mut(&mutex_id).ok_or("Mutex not found")?;

    match mutex.owner {
        Some(owner) => {
            if owner != pid {
                return Err("Operation not permitted: process does not own the lock".to_string());
            }
            // Release lock and wake up next waiter
            if !mutex.waiters.is_empty() {
                let next_owner = mutex.waiters.remove(0);
                mutex.owner = Some(next_owner);
                set_process_state(next_owner, ProcessState::Ready);
            } else {
                mutex.owner = None;
            }
            Ok(())
        }
        None => Err("Mutex is not locked".to_string()),
    }
}

pub fn get_mutexes() -> Vec<MutexInfo> {
    let table_lock = MUTEX_TABLE.lock().unwrap();
    if let Some(ref table) = *table_lock {
        table.values().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn detect_deadlock() -> Option<Vec<u32>> {
    let mutexes = get_mutexes();
    let mut adj = HashMap::new(); // PID -> PID (who owns the lock this PID is waiting for)
    
    for m in mutexes {
        if let Some(owner) = m.owner {
            for &waiter in &m.waiters {
                adj.insert(waiter, owner);
            }
        }
    }
    
    for &start in adj.keys() {
        let mut path = Vec::new();
        let mut curr = start;
        let mut visited = std::collections::HashSet::new();
        
        while adj.contains_key(&curr) {
            if visited.contains(&curr) {
                if let Some(pos) = path.iter().position(|&x| x == curr) {
                    return Some(path[pos..].to_vec());
                }
                break;
            }
            visited.insert(curr);
            path.push(curr);
            curr = adj[&curr];
        }
    }
    None
}

pub fn run_deadlock_simulation() -> Result<(), String> {
    // 1. Create three Chopstick mutexes
    let m1 = create_mutex("Chopstick 1")?;
    let m2 = create_mutex("Chopstick 2")?;
    let m3 = create_mutex("Chopstick 3")?;
    
    // 2. Create three philosopher processes
    let pid1 = super::task::create_process("Philosopher 1", super::task::ProcessPriority::Normal, Vec::new(), 1001)?;
    let pid2 = super::task::create_process("Philosopher 2", super::task::ProcessPriority::Normal, Vec::new(), 1001)?;
    let pid3 = super::task::create_process("Philosopher 3", super::task::ProcessPriority::Normal, Vec::new(), 1001)?;
    
    // 3. Philosophers pick up left chopstick
    lock_mutex(m1, pid1)?;
    lock_mutex(m2, pid2)?;
    lock_mutex(m3, pid3)?;
    
    // 4. Philosophers attempt to pick up right chopstick (all block -> DEADLOCK!)
    let _ = lock_mutex(m2, pid1)?;
    let _ = lock_mutex(m3, pid2)?;
    let _ = lock_mutex(m1, pid3)?;
    
    Ok(())
}

pub fn resolve_deadlock() -> Result<String, String> {
    if let Some(cycle) = detect_deadlock() {
        if let Some(&victim_pid) = cycle.first() {
            // Find which mutexes the victim held and release them
            let mut released_mutexes = Vec::new();
            {
                let mut table_lock = MUTEX_TABLE.lock().unwrap();
                if let Some(ref mut table) = *table_lock {
                    for m in table.values_mut() {
                        if m.owner == Some(victim_pid) {
                            released_mutexes.push(m.id);
                        }
                    }
                }
            }
            
            for mid in released_mutexes {
                let _ = unlock_mutex(mid, victim_pid);
            }
            
            // Kill the victim process
            kill_process(victim_pid)?;
            
            return Ok(format!("Killed PID {} to break the deadlock cycle.", victim_pid));
        }
    }
    Err("No deadlock detected".to_string())
}
