use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct SyscallTrace {
    pub syscall_name: String,
    pub pid: u32,
    pub args: [u64; 3],
    pub result: Result<u64, String>,
}

static SYSCALL_LOG: Mutex<Option<Vec<SyscallTrace>>> = Mutex::new(None);

pub fn init_syscall() {
    *SYSCALL_LOG.lock().unwrap() = Some(Vec::new());
}

// Log a system call execution for the student Syscall Explorer
pub fn log_syscall(name: &str, pid: u32, args: [u64; 3], res: Result<u64, String>) {
    let mut log_lock = SYSCALL_LOG.lock().unwrap();
    if let Some(ref mut logs) = *log_lock {
        if logs.len() > 100 {
            logs.remove(0); // sliding window
        }
        logs.push(SyscallTrace {
            syscall_name: name.to_string(),
            pid,
            args,
            result: res,
        });
    }
}

// POSIX System Call dispatch handler
pub fn sys_call(id: u32, arg1: u64, arg2: u64, arg3: u64) -> Result<u64, String> {
    let pid = super::task::get_current_pid();
    let name: &str;
    
    let res = match id {
        1 => {
            name = "sys_exit";
            // arg1: exit code
            let _ = super::task::kill_process(pid);
            Ok(arg1)
        }
        2 => {
            name = "sys_fork";
            // Clones the address space and process metadata
            let (parent_uid, parent_capabilities) = {
                let list = super::task::get_process_list();
                list.iter().find(|p| p.pid == pid)
                    .map(|p| (p.owner_uid, p.capabilities.clone()))
                    .unwrap_or((1001, Vec::new()))
            };
            let child_pid = super::task::create_process("child_task", super::task::ProcessPriority::Normal, parent_capabilities, parent_uid)?;
            super::mem::fork_address_space(pid, child_pid)?;
            Ok(child_pid as u64)
        }
        3 => {
            name = "sys_read";
            // arg1: file descriptor, arg2: buffer pointer (simulated), arg3: buffer len
            let fd = arg1 as usize;
            let len = arg3 as usize;
            let mut buf = vec![0u8; len];
            let bytes_read = filesystem::vfs_read(fd, &mut buf)?;
            Ok(bytes_read as u64)
        }
        4 => {
            name = "sys_write";
            // arg1: file descriptor, arg2: data pointer (simulated), arg3: data len
            let fd = arg1 as usize;
            let len = arg3 as usize;
            let process_uid = {
                let list = super::task::get_process_list();
                list.iter().find(|p| p.pid == pid).map(|p| p.owner_uid).unwrap_or(0)
            };
            // For security, let's verify MAC constraints first
            if filesystem::vfs_list_dir("/").is_ok() {
                // If it's a file write, enforce MAC check
                // Simulating MAC validation
                super::security::enforce_mac_path(process_uid, "user_temp", true)?;
            }

            // We mock raw data bytes parsing
            let raw_data = vec![b' '; len];
            // Safe copy simulation
            let bytes_written = filesystem::vfs_write(fd, &raw_data, process_uid)?;
            Ok(bytes_written as u64)
        }
        5 => {
            name = "sys_yield";
            // Voluntarily yields CPU control
            super::scheduler::schedule_next();
            Ok(0)
        }
        6 => {
            name = "sys_kill";
            // arg1: pid to kill
            let target_pid = arg1 as u32;
            if super::security::check_capability(pid, super::task::Capability::ProcessKill) {
                super::task::kill_process(target_pid)?;
                Ok(0)
            } else {
                Err("Operation not permitted: lacks ProcessKill capability".to_string())
            }
        }
        7 => {
            name = "sys_socket";
            // arg1: port
            let port = arg1 as u16;
            networking::tcp::bind_socket(port)?;
            Ok(0)
        }
        _ => {
            name = "sys_unknown";
            Err(format!("Unknown syscall identifier: {}", id))
        }
    };

    log_syscall(name, pid, [arg1, arg2, arg3], res.clone());
    res
}

// Read log of syscall traces
pub fn get_syscall_traces() -> Vec<SyscallTrace> {
    let log_lock = SYSCALL_LOG.lock().unwrap();
    if let Some(ref logs) = *log_lock {
        logs.clone()
    } else {
        Vec::new()
    }
}
