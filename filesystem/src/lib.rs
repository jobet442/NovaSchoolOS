pub mod novafs;
pub mod fat32;
pub mod ext2;

use std::sync::Mutex;
use std::collections::HashMap;

// Active mount points mapping
// For example: "/mnt/fat32" -> FAT32 type, "/mnt/ext2" -> EXT2 type
#[derive(Debug, Clone)]
pub enum MountType {
    NovaFS,
    FAT32,
    EXT2,
}

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub path: String,
    pub mtype: MountType,
}

// Represent an open file descriptor
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub path: String,
    pub inode_id: Option<u32>,
    pub offset: usize,
    pub is_dir: bool,
    pub readable: bool,
    pub writable: bool,
    pub mount_type: MountType,
}

// System-wide open file descriptors table
static FILE_DESCRIPTOR_TABLE: Mutex<Option<HashMap<usize, FileDescriptor>>> = Mutex::new(None);
static NEXT_FD: Mutex<usize> = Mutex::new(3); // 0: stdin, 1: stdout, 2: stderr

// Mount points register
static MOUNT_POINTS: Mutex<Vec<MountPoint>> = Mutex::new(Vec::new());

// VFS Initialization
pub fn init_vfs() {
    novafs::init_novafs();

    let mut fd_table = HashMap::new();
    // Pre-populate standard I/O (stub descriptors)
    fd_table.insert(0, FileDescriptor {
        path: "/dev/stdin".to_string(),
        inode_id: None,
        offset: 0,
        is_dir: false,
        readable: true,
        writable: false,
        mount_type: MountType::NovaFS,
    });
    fd_table.insert(1, FileDescriptor {
        path: "/dev/stdout".to_string(),
        inode_id: None,
        offset: 0,
        is_dir: false,
        readable: false,
        writable: true,
        mount_type: MountType::NovaFS,
    });
    fd_table.insert(2, FileDescriptor {
        path: "/dev/stderr".to_string(),
        inode_id: None,
        offset: 0,
        is_dir: false,
        readable: false,
        writable: true,
        mount_type: MountType::NovaFS,
    });
    *FILE_DESCRIPTOR_TABLE.lock().unwrap() = Some(fd_table);

    // Initial mounts
    let mut mounts = MOUNT_POINTS.lock().unwrap();
    mounts.clear();
    mounts.push(MountPoint { path: "/".to_string(), mtype: MountType::NovaFS });
    mounts.push(MountPoint { path: "/mnt/fat32".to_string(), mtype: MountType::FAT32 });
    mounts.push(MountPoint { path: "/mnt/ext2".to_string(), mtype: MountType::EXT2 });
}

// VFS Open API
pub fn vfs_open(path: &str, create: bool, write_mode: bool, uid: u32) -> Result<usize, String> {
    let clean_path = path.trim().replace("\\", "/");
    
    // Resolve mount type
    let mut mtype = MountType::NovaFS;
    let mut relative_path = clean_path.clone();

    let mounts = MOUNT_POINTS.lock().unwrap();
    for mount in mounts.iter() {
        if mount.path != "/" && clean_path.starts_with(&mount.path) {
            mtype = mount.mtype.clone();
            relative_path = clean_path[mount.path.len()..].trim_start_matches('/').to_string();
            break;
        }
    }

    match mtype {
        MountType::NovaFS => {
            // NovaFS Path Resolution
            // For simplicity, we search the directory entries of the root directory.
            // A full OS does multi-directory traversal. We will implement simple depth-1 directory lookup,
            // e.g. "student_work/lab1.txt". Let's resolve it.
            let parts: Vec<&str> = relative_path.split('/').filter(|s| !s.is_empty()).collect();
            let mut curr_inode = 0; // Root is inode 0

            for &part in &parts {
                let entries = novafs::read_directory_entries(curr_inode)?;
                let found = entries.iter().find(|e| e.name == part);
                if let Some(entry) = found {
                    curr_inode = entry.inode_id;
                } else {
                    if create && part == *parts.last().unwrap() {
                        // Create file in parent directory
                        let new_inode = novafs::allocate_inode(false, uid, 100, 0o644)?;
                        novafs::add_dir_entry(curr_inode, part, new_inode)?;
                        curr_inode = new_inode;
                    } else {
                        return Err(format!("VFS path not found: {}", clean_path));
                    }
                }
            }

            // Check permissions
            let allowed = novafs::check_permission(curr_inode, uid, write_mode)?;
            if !allowed {
                return Err("Permission denied".to_string());
            }

            let inode = novafs::read_inode_disk(curr_inode)?;
            
            // Allocate FD
            let mut fd_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
            let fd_table = fd_lock.as_mut().ok_or("FD table not initialized")?;
            let mut next_fd_lock = NEXT_FD.lock().unwrap();
            let fd = *next_fd_lock;
            
            fd_table.insert(fd, FileDescriptor {
                path: clean_path,
                inode_id: Some(curr_inode),
                offset: 0,
                is_dir: inode.is_directory,
                readable: true,
                writable: write_mode,
                mount_type: MountType::NovaFS,
            });
            
            *next_fd_lock += 1;
            Ok(fd)
        }
        MountType::FAT32 => {
            if write_mode {
                return Err("FAT32 partition is mounted read-only".to_string());
            }
            let fat = fat32::Fat32Simulator::new();
            let files = fat.list_files();
            if files.iter().any(|(name, _)| name == &relative_path || relative_path.is_empty()) {
                // Found
                let mut fd_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
                let fd_table = fd_lock.as_mut().unwrap();
                let mut next_fd_lock = NEXT_FD.lock().unwrap();
                let fd = *next_fd_lock;
                
                fd_table.insert(fd, FileDescriptor {
                    path: clean_path,
                    inode_id: None, // Simulated
                    offset: 0,
                    is_dir: relative_path.is_empty(),
                    readable: true,
                    writable: false,
                    mount_type: MountType::FAT32,
                });
                *next_fd_lock += 1;
                Ok(fd)
            } else {
                Err(format!("File not found on FAT32 partition: {}", relative_path))
            }
        }
        MountType::EXT2 => {
            if write_mode {
                return Err("EXT2 simulator partition is mounted read-only".to_string());
            }
            let ext = ext2::Ext2Simulator::new();
            let inodes = ext.list_inodes();
            if let Some((inode_id, _, _)) = inodes.iter().find(|(_, name, _)| name == &relative_path || relative_path.is_empty()) {
                let mut fd_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
                let fd_table = fd_lock.as_mut().unwrap();
                let mut next_fd_lock = NEXT_FD.lock().unwrap();
                let fd = *next_fd_lock;
                
                fd_table.insert(fd, FileDescriptor {
                    path: clean_path,
                    inode_id: Some(*inode_id),
                    offset: 0,
                    is_dir: relative_path.is_empty(),
                    readable: true,
                    writable: false,
                    mount_type: MountType::EXT2,
                });
                *next_fd_lock += 1;
                Ok(fd)
            } else {
                Err(format!("File not found on EXT2 partition: {}", relative_path))
            }
        }
    }
}

// VFS Read API
pub fn vfs_read(fd: usize, buf: &mut [u8]) -> Result<usize, String> {
    let mut fd_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
    let fd_table = fd_lock.as_mut().ok_or("FD table not initialized")?;
    let desc = fd_table.get_mut(&fd).ok_or("Invalid file descriptor")?;

    if !desc.readable {
        return Err("File descriptor not readable".to_string());
    }

    match desc.mount_type {
        MountType::NovaFS => {
            if let Some(inode_id) = desc.inode_id {
                let bytes_read = novafs::read_file_data(inode_id, desc.offset, buf)?;
                desc.offset += bytes_read;
                Ok(bytes_read)
            } else {
                // Handling standard input character read
                if desc.path == "/dev/stdin" {
                    if let Some(event) = drivers::input::read_key_event() {
                        if let drivers::input::KeyCode::Char(c) = event.code {
                            buf[0] = c as u8;
                            return Ok(1);
                        } else if event.code == drivers::input::KeyCode::Enter {
                            buf[0] = b'\n';
                            return Ok(1);
                        }
                    }
                    Ok(0) // non-blocking returns 0 if empty
                } else {
                    Err("Invalid inode read".to_string())
                }
            }
        }
        MountType::FAT32 => {
            let relative_path = desc.path.trim_start_matches("/mnt/fat32").trim_start_matches('/');
            let fat = fat32::Fat32Simulator::new();
            let content = fat.read_file(relative_path)?;
            let content_bytes = content.as_bytes();
            if desc.offset >= content_bytes.len() {
                return Ok(0);
            }
            let size = std::cmp::min(buf.len(), content_bytes.len() - desc.offset);
            buf[0..size].copy_from_slice(&content_bytes[desc.offset..(desc.offset + size)]);
            desc.offset += size;
            Ok(size)
        }
        MountType::EXT2 => {
            if let Some(inode_id) = desc.inode_id {
                let ext = ext2::Ext2Simulator::new();
                let content = ext.read_file_by_inode(inode_id)?;
                let content_bytes = content.as_bytes();
                if desc.offset >= content_bytes.len() {
                    return Ok(0);
                }
                let size = std::cmp::min(buf.len(), content_bytes.len() - desc.offset);
                buf[0..size].copy_from_slice(&content_bytes[desc.offset..(desc.offset + size)]);
                desc.offset += size;
                Ok(size)
            } else {
                Err("No inode for EXT2 VFS descriptor".to_string())
            }
        }
    }
}

// VFS Write API
pub fn vfs_write(fd: usize, buf: &[u8], uid: u32) -> Result<usize, String> {
    let mut fd_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
    let fd_table = fd_lock.as_mut().ok_or("FD table not initialized")?;
    let desc = fd_table.get_mut(&fd).ok_or("Invalid file descriptor")?;

    if !desc.writable {
        return Err("File descriptor not writable".to_string());
    }

    match desc.mount_type {
        MountType::NovaFS => {
            if let Some(inode_id) = desc.inode_id {
                novafs::write_file_data(inode_id, desc.offset, buf, uid)?;
                desc.offset += buf.len();
                Ok(buf.len())
            } else {
                // Handling standard output character print
                if desc.path == "/dev/stdout" || desc.path == "/dev/stderr" {
                    if let Ok(s) = std::str::from_utf8(buf) {
                        drivers::vga_print!("{}", s);
                        Ok(buf.len())
                    } else {
                        Err("Invalid UTF8 output string".to_string())
                    }
                } else {
                    Err("Invalid inode write".to_string())
                }
            }
        }
        MountType::FAT32 | MountType::EXT2 => {
            Err("Cannot write: external filesystem mounted read-only".to_string())
        }
    }
}

// VFS Close FD
pub fn vfs_close(fd: usize) -> Result<(), String> {
    let mut fd_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
    let fd_table = fd_lock.as_mut().ok_or("FD table not initialized")?;
    if fd_table.remove(&fd).is_some() {
        Ok(())
    } else {
        Err("File descriptor not found".to_string())
    }
}

// VFS Make Directory
pub fn vfs_mkdir(path: &str, permissions: u16, uid: u32) -> Result<(), String> {
    let clean_path = path.trim().replace("\\", "/");
    if clean_path.starts_with("/mnt") {
        return Err("Cannot create directory inside mounted partitions".to_string());
    }

    let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();
    let mut curr_inode = 0; // Root is inode 0

    for i in 0..parts.len() {
        let part = parts[i];
        let entries = novafs::read_directory_entries(curr_inode)?;
        let found = entries.iter().find(|e| e.name == part);
        if let Some(entry) = found {
            curr_inode = entry.inode_id;
        } else {
            if i == parts.len() - 1 {
                // Create directory inode
                let new_inode = novafs::allocate_inode(true, uid, 100, permissions)?;
                novafs::add_dir_entry(curr_inode, part, new_inode)?;
                // Initialize directory block with '.' and '..'
                novafs::add_dir_entry(new_inode, ".", new_inode)?;
                novafs::add_dir_entry(new_inode, "..", curr_inode)?;
                curr_inode = new_inode;
            } else {
                return Err(format!("Parent directory not found for: {}", part));
            }
        }
    }
    Ok(())
}

// Helper to list directories inside VFS (for the ls utility)
pub fn vfs_list_dir(path: &str) -> Result<Vec<(String, u32, bool, u16)>, String> {
    let clean_path = path.trim().replace("\\", "/");
    let mut mtype = MountType::NovaFS;
    let mut relative_path = clean_path.clone();

    let mounts = MOUNT_POINTS.lock().unwrap();
    for mount in mounts.iter() {
        if mount.path != "/" && clean_path.starts_with(&mount.path) {
            mtype = mount.mtype.clone();
            relative_path = clean_path[mount.path.len()..].trim_start_matches('/').to_string();
            break;
        }
    }

    match mtype {
        MountType::NovaFS => {
            let parts: Vec<&str> = relative_path.split('/').filter(|s| !s.is_empty()).collect();
            let mut curr_inode = 0; // Root is inode 0

            for &part in &parts {
                let entries = novafs::read_directory_entries(curr_inode)?;
                let found = entries.iter().find(|e| e.name == part);
                if let Some(entry) = found {
                    curr_inode = entry.inode_id;
                } else {
                    return Err(format!("Directory path not found: {}", clean_path));
                }
            }

            let entries = novafs::read_directory_entries(curr_inode)?;
            let mut results = Vec::new();
            for entry in entries {
                if let Ok(inode) = novafs::read_inode_disk(entry.inode_id) {
                    results.push((entry.name, inode.size, inode.is_directory, inode.permissions));
                }
            }
            Ok(results)
        }
        MountType::FAT32 => {
            let fat = fat32::Fat32Simulator::new();
            let files = fat.list_files();
            let results = files.into_iter().map(|(name, size)| {
                (name, size as u32, false, 0o444)
            }).collect();
            Ok(results)
        }
        MountType::EXT2 => {
            let ext = ext2::Ext2Simulator::new();
            let inodes = ext.list_inodes();
            let results = inodes.into_iter().map(|(_, name, perm)| {
                (name, 0u32, false, perm)
            }).collect();
            Ok(results)
        }
    }
}

// Expose active VFS mount points for visualizer
pub fn get_vfs_mount_points() -> Vec<MountPoint> {
    let mounts = MOUNT_POINTS.lock().unwrap();
    mounts.clone()
}

// Expose open VFS file descriptors for visualizer
pub fn get_vfs_open_files() -> Vec<(usize, FileDescriptor)> {
    let fd_table_lock = FILE_DESCRIPTOR_TABLE.lock().unwrap();
    if let Some(ref fd_table) = *fd_table_lock {
        let mut list: Vec<(usize, FileDescriptor)> = fd_table.iter().map(|(&fd, desc)| (fd, desc.clone())).collect();
        list.sort_by_key(|item| item.0);
        list
    } else {
        Vec::new()
    }
}
