use drivers::init_drivers;
use filesystem::{init_vfs, vfs_open, vfs_write, vfs_read, vfs_close, vfs_mkdir};
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_vfs_mounts_and_file_io() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    // Init drivers and mock disk
    init_drivers();
    init_vfs();

    // Create intermediate directories first
    vfs_mkdir("/students", 0o755, 0).expect("Should create /students");
    vfs_mkdir("/students/student1001", 0o755, 1001).expect("Should create student workspace");

    // Create folder inside student workspace
    vfs_mkdir("/students/student1001/lab1", 0o755, 1001).expect("Should create directory");

    // Open file inside directory
    let fd = vfs_open("/students/student1001/lab1/report.txt", true, true, 1001)
        .expect("Should open/create file");

    let text = "Lab completed: Successful.";
    vfs_write(fd, text.as_bytes(), 1001).expect("Should write file data");
    vfs_close(fd).unwrap();

    // Read back file
    let fd_read = vfs_open("/students/student1001/lab1/report.txt", false, false, 1001).unwrap();
    let mut buffer = vec![0u8; 100];
    let bytes_read = vfs_read(fd_read, &mut buffer).unwrap();
    vfs_close(fd_read).unwrap();

    let read_str = std::str::from_utf8(&buffer[0..bytes_read]).unwrap();
    assert_eq!(read_str, text);
}

#[test]
fn test_vfs_quota_limit() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    init_drivers();
    init_vfs();

    // Create intermediate directories first
    vfs_mkdir("/students", 0o755, 0).expect("Should create /students");
    vfs_mkdir("/students/student1001", 0o755, 1001).expect("Should create student workspace");

    let fd = vfs_open("/students/student1001/report_heavy.txt", true, true, 1001).unwrap();
    
    // Write data exceeding student quota limit (2000 bytes)
    let heavy_data = vec![0u8; 2500]; // 2.5 KB
    let write_res = vfs_write(fd, &heavy_data, 1001);
    vfs_close(fd).unwrap();

    // Verification check: write operation should fail on quota limits check
    assert!(write_res.is_err());
    assert!(write_res.err().unwrap().to_lowercase().contains("quota exceeded"));
}
