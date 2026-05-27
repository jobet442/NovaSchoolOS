use kernel::task::{init_task, create_process, ProcessPriority};
use kernel::ipc::{init_ipc, create_queue, send_message, recv_message, get_queues};
use kernel::mutex::{init_mutex, create_mutex, lock_mutex, unlock_mutex, detect_deadlock, resolve_deadlock, get_mutexes};
use kernel::mem::init_mem;
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_message_queue() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    init_ipc();

    // Create queue
    assert!(create_queue("test_q").is_ok());
    // Duplicate queue should error
    assert!(create_queue("test_q").is_err());

    // Send messages
    assert!(send_message("test_q", "hello".to_string()).is_ok());
    assert!(send_message("test_q", "world".to_string()).is_ok());

    let queues = get_queues();
    let q = queues.iter().find(|(name, _, _)| name == "test_q").unwrap();
    assert_eq!(q.1, 2); // 2 messages pending

    // Receive messages
    assert_eq!(recv_message("test_q").unwrap(), Some("hello".to_string()));
    assert_eq!(recv_message("test_q").unwrap(), Some("world".to_string()));
    assert_eq!(recv_message("test_q").unwrap(), None);
}

#[test]
fn test_mutex_concurrency() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    init_mem();
    init_task();
    init_mutex();

    let pid1 = create_process("process1", ProcessPriority::Normal, Vec::new(), 1001).unwrap();
    let pid2 = create_process("process2", ProcessPriority::Normal, Vec::new(), 1001).unwrap();

    let m1 = create_mutex("resource1").unwrap();

    // PID 1 acquires lock
    assert_eq!(lock_mutex(m1, pid1).unwrap(), true); // immediately acquired

    // PID 2 tries to acquire lock (blocks)
    assert_eq!(lock_mutex(m1, pid2).unwrap(), false); // blocked

    // Verify PID 2 is Blocked and in wait queue
    let mutexes = get_mutexes();
    let m_info = mutexes.iter().find(|m| m.id == m1).unwrap();
    assert_eq!(m_info.owner, Some(pid1));
    assert_eq!(m_info.waiters, vec![pid2]);

    // PID 1 releases lock, PID 2 should acquire it and wake up
    assert!(unlock_mutex(m1, pid1).is_ok());

    let mutexes_after = get_mutexes();
    let m_info_after = mutexes_after.iter().find(|m| m.id == m1).unwrap();
    assert_eq!(m_info_after.owner, Some(pid2));
    assert!(m_info_after.waiters.is_empty());
}

#[test]
fn test_deadlock_detection_and_resolution() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    init_mem();
    init_task();
    init_mutex();

    let pid1 = create_process("philosopher1", ProcessPriority::Normal, Vec::new(), 1001).unwrap();
    let pid2 = create_process("philosopher2", ProcessPriority::Normal, Vec::new(), 1001).unwrap();

    let chopstick1 = create_mutex("c1").unwrap();
    let chopstick2 = create_mutex("c2").unwrap();

    // P1 locks c1, P2 locks c2
    assert_eq!(lock_mutex(chopstick1, pid1).unwrap(), true);
    assert_eq!(lock_mutex(chopstick2, pid2).unwrap(), true);

    // No deadlock yet
    assert!(detect_deadlock().is_none());

    // P1 tries to lock c2 (blocks), P2 tries to lock c1 (blocks) -> Deadlock!
    assert_eq!(lock_mutex(chopstick2, pid1).unwrap(), false);
    assert_eq!(lock_mutex(chopstick1, pid2).unwrap(), false);

    let cycle = detect_deadlock().expect("Deadlock should be detected");
    assert!(cycle.contains(&pid1));
    assert!(cycle.contains(&pid2));

    // Resolve deadlock
    let res = resolve_deadlock().expect("Deadlock resolution should succeed");
    assert!(res.contains("Killed PID"));

    // Deadlock cycle should be broken
    assert!(detect_deadlock().is_none());
}
