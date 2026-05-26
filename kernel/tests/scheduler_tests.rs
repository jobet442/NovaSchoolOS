use kernel::task::{init_task, create_process, ProcessPriority, set_process_state, ProcessState};
use kernel::scheduler::{set_scheduling_policy, SchedulingPolicy, schedule_next};
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_round_robin_scheduler() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_task();
    set_scheduling_policy(SchedulingPolicy::RoundRobin);

    // Create active processes
    let pid1 = create_process("task_a", ProcessPriority::Normal, Vec::new(), 1001).unwrap();
    let pid2 = create_process("task_b", ProcessPriority::Normal, Vec::new(), 1001).unwrap();

    set_process_state(pid1, ProcessState::Ready);
    set_process_state(pid2, ProcessState::Ready);

    // Schedule next
    let next1 = schedule_next().unwrap();
    let next2 = schedule_next().unwrap();

    // Verify it cycles round-robin
    assert_ne!(next1, next2);
}

#[test]
fn test_priority_scheduler() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_task();
    set_scheduling_policy(SchedulingPolicy::Priority);

    let pid_low = create_process("low_task", ProcessPriority::Low, Vec::new(), 1001).unwrap();
    let pid_high = create_process("high_task", ProcessPriority::High, Vec::new(), 1001).unwrap();

    set_process_state(pid_low, ProcessState::Ready);
    set_process_state(pid_high, ProcessState::Ready);

    // Schedule next should pick high priority process
    let selected = schedule_next().unwrap();
    assert_eq!(selected, pid_high);
}
