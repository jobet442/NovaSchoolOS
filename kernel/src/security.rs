use std::sync::Mutex;
use super::task::{Capability, get_process_list};

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub timestamp: u64,
    pub pid: u32,
    pub user_uid: u32,
    pub action: String,
    pub status: String, // "ALLOWED", "DENIED"
    pub details: String,
}

static AUDIT_LOGS: Mutex<Option<Vec<AuditLogEntry>>> = Mutex::new(None);

pub fn init_security() {
    *AUDIT_LOGS.lock().unwrap() = Some(Vec::new());
}

// Log a security event
pub fn log_security_event(pid: u32, uid: u32, action: &str, status: &str, details: &str) {
    let mut logs_lock = AUDIT_LOGS.lock().unwrap();
    if let Some(ref mut logs) = *logs_lock {
        if logs.len() > 500 {
            logs.remove(0); // circular buffer
        }
        logs.push(AuditLogEntry {
            timestamp: 98765432, // simulated timestamp
            pid,
            user_uid: uid,
            action: action.to_string(),
            status: status.to_string(),
            details: details.to_string(),
        });
    }
}

// Validate if process is allowed to perform operation
pub fn check_capability(pid: u32, cap: Capability) -> bool {
    let processes = get_process_list();
    if let Some(p) = processes.iter().find(|p| p.pid == pid) {
        // Root uid is superuser, always allowed
        if p.owner_uid == 0 {
            return true;
        }
        let has_cap = p.capabilities.contains(&cap);
        if !has_cap {
            log_security_event(
                pid,
                p.owner_uid,
                &format!("CAPABILITY_CHECK:{:?}", cap),
                "DENIED",
                &format!("Process '{}' lack capabilities", p.name)
            );
        }
        return has_cap;
    }
    false
}

// Mandatory Access Control (MAC) enforcement
// Restricts student users from modifying files outside /students/<uid> or /shared
pub fn enforce_mac_path(uid: u32, path: &str, write_requested: bool) -> Result<(), String> {
    // Root uid bypasses MAC
    if uid == 0 {
        return Ok(());
    }

    let clean_path = path.trim().replace("\\", "/");
    
    // Immutable system directories rule
    if write_requested && (clean_path.starts_with("/bin") || clean_path.starts_with("/etc") || clean_path.starts_with("/boot")) {
        log_security_event(0, uid, "WRITE_SYSTEM_DIR", "DENIED", &format!("Attempt to write system directory: {}", clean_path));
        return Err("MAC Violation: Writing to immutable system directories is forbidden".to_string());
    }

    // Student workspaces isolation rule
    if clean_path.starts_with("/students") {
        let expected_subdir = format!("/students/student{}", uid);
        if !clean_path.starts_with(&expected_subdir) {
            log_security_event(0, uid, "MAC_VIOLATION", "DENIED", &format!("Attempt to cross-access workspace: {}", clean_path));
            return Err("MAC Violation: Cannot access other student workspace directory".to_string());
        }
    }

    Ok(())
}

// Get copy of security logs for Visualizer
pub fn get_security_audit_logs() -> Vec<AuditLogEntry> {
    let logs_lock = AUDIT_LOGS.lock().unwrap();
    if let Some(ref logs) = *logs_lock {
        logs.clone()
    } else {
        Vec::new()
    }
}
