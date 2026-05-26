use kernel::mem::{init_mem, allocate_frame, map_page, fork_address_space, handle_page_fault, get_memory_snapshot, FrameOwner, free_process_address_space};
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_frame_allocator() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_mem();
    
    // Allocate frame for PID 101
    let frame = allocate_frame(101).expect("Should allocate frame");
    assert!(frame >= 4096); // Lower frames are reserved for Kernel

    let (frames, _) = get_memory_snapshot();
    assert_eq!(frames[frame as usize], FrameOwner::Process(101));
}

#[test]
fn test_copy_on_write_fork() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_mem();

    let frame = allocate_frame(101).unwrap();
    map_page(101, 0x200000, frame, true).unwrap(); // Writable page

    // Fork to PID 102
    fork_address_space(101, 102).unwrap();

    let (frames, _) = get_memory_snapshot();
    // Frame should be marked as COW
    assert_eq!(frames[frame as usize], FrameOwner::Cow(101));

    // Handle write fault on PID 102 (COW resolver)
    handle_page_fault(102, 0x200000, true).unwrap();

    let (updated_frames, _) = get_memory_snapshot();
    // Frame should be duplicated: parent retains old, child gets new frame
    assert_eq!(updated_frames[frame as usize], FrameOwner::Process(101));
}

#[test]
fn test_free_process_address_space() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_mem();

    let frame1 = allocate_frame(101).unwrap();
    map_page(101, 0x100000, frame1, true).unwrap();

    let frame2 = allocate_frame(101).unwrap();
    map_page(101, 0x200000, frame2, true).unwrap();

    // Fork to PID 102
    fork_address_space(101, 102).unwrap();

    // Both frame1 and frame2 should be COW
    let (frames_before, _) = get_memory_snapshot();
    assert_eq!(frames_before[frame1 as usize], FrameOwner::Cow(101));
    assert_eq!(frames_before[frame2 as usize], FrameOwner::Cow(101));

    // Free child PID 102 address space
    free_process_address_space(102).unwrap();

    // The frames should revert to Process(101) and become writable again
    let (frames_after, _) = get_memory_snapshot();
    assert_eq!(frames_after[frame1 as usize], FrameOwner::Process(101));
    assert_eq!(frames_after[frame2 as usize], FrameOwner::Process(101));

    // Now free parent PID 101 address space
    free_process_address_space(101).unwrap();

    let (frames_final, _) = get_memory_snapshot();
    assert_eq!(frames_final[frame1 as usize], FrameOwner::Free);
    assert_eq!(frames_final[frame2 as usize], FrameOwner::Free);
}
