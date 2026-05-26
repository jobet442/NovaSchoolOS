use std::sync::Mutex;
use super::task::{Process, ProcessState, ProcessPriority, get_current_pid, set_current_pid, set_process_state, get_process_list};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    RoundRobin,
    Priority,
    RealTime,
}

static ACTIVE_POLICY: Mutex<SchedulingPolicy> = Mutex::new(SchedulingPolicy::RoundRobin);

pub fn set_scheduling_policy(policy: SchedulingPolicy) {
    *ACTIVE_POLICY.lock().unwrap() = policy;
}

pub fn get_scheduling_policy() -> SchedulingPolicy {
    *ACTIVE_POLICY.lock().unwrap()
}

// Selects the next process to run based on the active policy
pub fn schedule_next() -> Option<u32> {
    let current_pid = get_current_pid();
    let processes = get_process_list();

    // Only consider ready processes
    let ready_pids: Vec<u32> = processes.iter()
        .filter(|p| p.state == ProcessState::Ready && p.pid != 0)
        .map(|p| p.pid)
        .collect();

    if ready_pids.is_empty() {
        // Fall back to idle process (PID 0) or current running process if still runnable
        if let Some(curr) = processes.iter().find(|p| p.pid == current_pid) {
            if curr.state == ProcessState::Running {
                return Some(current_pid);
            }
        }
        return Some(0); // Run idle
    }

    let policy = get_scheduling_policy();
    let next_pid = match policy {
        SchedulingPolicy::RoundRobin => {
            // Find next ready PID in a circular queue fashion
            let curr_idx = ready_pids.iter().position(|&id| id == current_pid).unwrap_or(ready_pids.len() - 1);
            let next_idx = (curr_idx + 1) % ready_pids.len();
            ready_pids[next_idx]
        }
        SchedulingPolicy::Priority => {
            // Select process with the highest priority level
            let mut highest_p: Option<&Process> = None;
            for p in &processes {
                if p.state == ProcessState::Ready && p.pid != 0 {
                    match highest_p {
                        None => highest_p = Some(p),
                        Some(hp) => {
                            if (p.priority as u8) > (hp.priority as u8) {
                                highest_p = Some(p);
                            }
                        }
                    }
                }
            }
            highest_p.map(|p| p.pid).unwrap_or(0)
        }
        SchedulingPolicy::RealTime => {
            // Priority scheduling but strictly enforces RealTime priority preemption
            let rt_pids: Vec<u32> = processes.iter()
                .filter(|p| p.state == ProcessState::Ready && p.priority == ProcessPriority::RealTime)
                .map(|p| p.pid)
                .collect();

            if !rt_pids.is_empty() {
                // If there are RealTime processes, scheduling priority goes to them round-robin
                let curr_idx = rt_pids.iter().position(|&id| id == current_pid).unwrap_or(rt_pids.len() - 1);
                let next_idx = (curr_idx + 1) % rt_pids.len();
                rt_pids[next_idx]
            } else {
                // Otherwise fallback to normal RoundRobin
                let curr_idx = ready_pids.iter().position(|&id| id == current_pid).unwrap_or(ready_pids.len() - 1);
                let next_idx = (curr_idx + 1) % ready_pids.len();
                ready_pids[next_idx]
            }
        }
    };

    // Preempt old running process
    if current_pid != next_pid {
        set_process_state(current_pid, ProcessState::Ready);
        set_process_state(next_pid, ProcessState::Running);
        set_current_pid(next_pid);
    }

    // Accumulate ticks for run times
    accumulate_cpu_ticks(next_pid);

    Some(next_pid)
}

fn accumulate_cpu_ticks(pid: u32) {
    let _list_copy = super::task::get_process_list(); // fetch table
    // For simplicity, we directly update ticks in task manager.
    // In our task.rs we exported get_process_list as a copy, so we will implement
    // tick increment in task table directly.
    increment_task_ticks(pid);
}

// Helper to increment CPU ticks directly inside task manager
pub fn increment_task_ticks(pid: u32) {
    // Acquire task manager lock and increment
    // Since task table is static Mutex, let's write a small accessor
    let mut lock = super::task::drv_process_table();
    if let Some(ref mut table) = *lock {
        if let Some(p) = table.get_mut(&pid) {
            p.cpu_ticks += 1;
        }
    }
}
