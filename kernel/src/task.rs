use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Ready,
    Blocked,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPriority {
    RealTime = 3,
    High = 2,
    Normal = 1,
    Low = 0,
}

// Capability permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    SysAdmin,
    FileRead,
    FileWrite,
    NetworkRaw,
    ProcessKill,
}

// Simulated CPU registers representing thread context
#[derive(Debug, Clone, Default)]
pub struct CpuContext {
    pub rip: u64,
    pub rsp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rdi: u64,
    pub rsi: u64,
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub priority: ProcessPriority,
    pub capabilities: Vec<Capability>,
    pub file_descriptors: Vec<usize>,
    pub context: CpuContext,
    pub owner_uid: u32,
    pub cpu_ticks: u64,
}

static PROCESS_TABLE: Mutex<Option<HashMap<u32, Process>>> = Mutex::new(None);
static CURRENT_PID: Mutex<u32> = Mutex::new(0);
static NEXT_PID: Mutex<u32> = Mutex::new(1);

pub fn init_task() {
    let mut table = HashMap::new();
    
    // Create Idle Process (PID 0)
    table.insert(0, Process {
        pid: 0,
        name: "idle".to_string(),
        state: ProcessState::Running,
        priority: ProcessPriority::Low,
        capabilities: Vec::new(),
        file_descriptors: vec![0, 1, 2],
        context: CpuContext::default(),
        owner_uid: 0,
        cpu_ticks: 0,
    });

    *PROCESS_TABLE.lock().unwrap() = Some(table);
    *CURRENT_PID.lock().unwrap() = 0;
    *NEXT_PID.lock().unwrap() = 1;
}

pub fn create_process(name: &str, priority: ProcessPriority, capabilities: Vec<Capability>, uid: u32) -> Result<u32, String> {
    let mut table_lock = PROCESS_TABLE.lock().unwrap();
    let table = table_lock.as_mut().ok_or("Process table uninitialized")?;

    let mut next_pid = NEXT_PID.lock().unwrap();
    let pid = *next_pid;

    let p = Process {
        pid,
        name: name.to_string(),
        state: ProcessState::Ready,
        priority,
        capabilities,
        file_descriptors: vec![0, 1, 2], // inherits standard streams
        context: CpuContext {
            rip: 0x201000, // standard user start RIP
            rsp: 0x400000 + (pid as u64 * 0x10000), // separate stack
            rax: 0,
            rbx: 0,
            rdi: 0,
            rsi: 0,
        },
        owner_uid: uid,
        cpu_ticks: 0,
    };

    table.insert(pid, p);
    *next_pid += 1;
    Ok(pid)
}

pub fn set_process_state(pid: u32, state: ProcessState) {
    let mut table_lock = PROCESS_TABLE.lock().unwrap();
    if let Some(ref mut table) = *table_lock {
        if let Some(p) = table.get_mut(&pid) {
            p.state = state;
        }
    }
}

pub fn get_current_pid() -> u32 {
    *CURRENT_PID.lock().unwrap()
}

pub fn set_current_pid(pid: u32) {
    *CURRENT_PID.lock().unwrap() = pid;
}

pub fn get_process_list() -> Vec<Process> {
    let table_lock = PROCESS_TABLE.lock().unwrap();
    if let Some(ref table) = *table_lock {
        table.values().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn kill_process(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("Cannot kill idle process".to_string());
    }
    
    // Free its physical memory pages
    super::mem::free_process_address_space(pid)?;

    let mut table_lock = PROCESS_TABLE.lock().unwrap();
    if let Some(ref mut table) = *table_lock {
        if let Some(p) = table.get_mut(&pid) {
            p.state = ProcessState::Killed;
            return Ok(());
        }
    }
    Err("Process not found".to_string())
}

pub fn drv_process_table() -> std::sync::MutexGuard<'static, Option<HashMap<u32, Process>>> {
    PROCESS_TABLE.lock().unwrap()
}

