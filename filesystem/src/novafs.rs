use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

// Sector Layout Configuration
// Sector 0: Boot sector
// Sector 1: Superblock
// Sector 2..10: Inode block area
// Sector 11..15: Journal block area
// Sector 16..4095: Data block area

pub const MAGIC: u32 = 0x4E4F5641; // "NOVA"
pub const SUPERBLOCK_SECTOR: usize = 1;
pub const INODE_START_SECTOR: usize = 2;
pub const INODE_SECTORS: usize = 9;
pub const JOURNAL_START_SECTOR: usize = 11;
pub const JOURNAL_SECTORS: usize = 5;
pub const DATA_START_SECTOR: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Superblock {
    pub magic: u32,
    pub total_blocks: u32,
    pub free_blocks: u32,
    pub inode_count: u32,
    pub journal_ptr: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub id: u32,
    pub is_directory: bool,
    pub size: u32,
    pub owner_uid: u32,
    pub owner_gid: u32,
    pub permissions: u16, // e.g. 0o755
    pub direct_blocks: [u32; 6], // indexes to data sectors
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub inode_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub transaction_id: u32,
    pub inode_id: u32,
    pub operation: String, // "CREATE", "WRITE", "DELETE"
    pub data_offset: u32,
    pub committed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: u32,
    pub timestamp: u64,
    pub root_inode_copy: Inode,
    // Store simple backup of disk sectors representing the snapshot
    pub disk_backup: HashMap<usize, Vec<u8>>,
}

// Global active user quotas (UID -> Max Bytes allowed)
static USER_QUOTAS: Mutex<Option<HashMap<u32, u32>>> = Mutex::new(None);
// Global active user usage (UID -> Bytes currently used)
static USER_USAGES: Mutex<Option<HashMap<u32, u32>>> = Mutex::new(None);
// Active snapshots database
static SNAPSHOTS: Mutex<Option<HashMap<u32, Snapshot>>> = Mutex::new(None);

pub fn init_novafs() {
    let mut quotas = HashMap::new();
    quotas.insert(1001, 2000); // 2 KB limit for Student UID 1001
    quotas.insert(1002, 2000); // 2 KB limit for Student UID 1002
    *USER_QUOTAS.lock().unwrap() = Some(quotas);

    let mut usages = HashMap::new();
    usages.insert(1001, 0);
    usages.insert(1002, 0);
    *USER_USAGES.lock().unwrap() = Some(usages);

    *SNAPSHOTS.lock().unwrap() = Some(HashMap::new());

    // Format if necessary
    if let Err(_) = read_superblock_disk() {
        format_disk().unwrap();
    }
}

// In-memory buffer read/write helpers
fn write_superblock_disk(sb: &Superblock) -> Result<(), String> {
    let bytes = serde_json::to_vec(sb).map_err(|e| e.to_string())?;
    let mut sector = [0u8; 512];
    if bytes.len() > 512 {
        return Err("Superblock too large".to_string());
    }
    sector[0..bytes.len()].copy_from_slice(&bytes);
    drivers::disk::write_block(SUPERBLOCK_SECTOR, &sector)
}

fn read_superblock_disk() -> Result<Superblock, String> {
    let mut sector = [0u8; 512];
    drivers::disk::read_block(SUPERBLOCK_SECTOR, &mut sector)?;
    
    // Find trailing zeros and slice to parse JSON
    let end = sector.iter().position(|&x| x == 0).unwrap_or(512);
    if end == 0 {
        return Err("Empty superblock sector".to_string());
    }
    let sb: Superblock = serde_json::from_slice(&sector[0..end]).map_err(|e| e.to_string())?;
    if sb.magic != MAGIC {
        return Err("Invalid magic number".to_string());
    }
    Ok(sb)
}

// Format the virtual disk with a fresh NovaFS filesystem
pub fn format_disk() -> Result<(), String> {
    let sb = Superblock {
        magic: MAGIC,
        total_blocks: drivers::disk::DISK_SECTORS as u32,
        free_blocks: (drivers::disk::DISK_SECTORS - DATA_START_SECTOR) as u32,
        inode_count: 0,
        journal_ptr: 0,
    };
    write_superblock_disk(&sb)?;

    // Create root directory inode (Inode ID: 0)
    let root_inode = Inode {
        id: 0,
        is_directory: true,
        size: 0,
        owner_uid: 0, // root
        owner_gid: 0, // root
        permissions: 0o755,
        direct_blocks: [0; 6],
    };
    write_inode_disk(0, &root_inode)?;

    // Set superblock count
    let mut sb = read_superblock_disk()?;
    sb.inode_count = 1;
    write_superblock_disk(&sb)?;

    Ok(())
}

pub fn write_inode_disk(id: u32, inode: &Inode) -> Result<(), String> {
    // Inodes are stored in the Inode sector area.
    // Calculate sector offset and index in sector. Let's serialize Inode to json
    // and write to the corresponding sector.
    let sector_offset = (id as usize * 128) / 512; // Assume max 4 inodes per sector
    let target_sector = INODE_START_SECTOR + sector_offset;
    if target_sector >= JOURNAL_START_SECTOR {
        return Err("Inode ID out of limits".to_string());
    }

    let mut sector = [0u8; 512];
    let _ = drivers::disk::read_block(target_sector, &mut sector); // Load current contents

    let idx = (id as usize) % 4;
    let serialized = serde_json::to_vec(inode).map_err(|e| e.to_string())?;
    if serialized.len() > 127 {
        return Err("Inode too complex to serialize in block slot".to_string());
    }

    // Write size byte followed by data
    let slot_start = idx * 128;
    sector[slot_start] = serialized.len() as u8;
    sector[(slot_start + 1)..(slot_start + 1 + serialized.len())].copy_from_slice(&serialized);

    drivers::disk::write_block(target_sector, &sector)?;
    Ok(())
}

pub fn read_inode_disk(id: u32) -> Result<Inode, String> {
    let sector_offset = (id as usize * 128) / 512;
    let target_sector = INODE_START_SECTOR + sector_offset;
    if target_sector >= JOURNAL_START_SECTOR {
        return Err("Inode ID out of limits".to_string());
    }

    let mut sector = [0u8; 512];
    drivers::disk::read_block(target_sector, &mut sector)?;

    let idx = (id as usize) % 4;
    let slot_start = idx * 128;
    let len = sector[slot_start] as usize;
    if len == 0 {
        return Err(format!("Inode {} does not exist", id));
    }

    let inode: Inode = serde_json::from_slice(&sector[(slot_start + 1)..(slot_start + 1 + len)])
        .map_err(|e| e.to_string())?;
    Ok(inode)
}

// Inode allocation
pub fn allocate_inode(is_dir: bool, uid: u32, gid: u32, permissions: u16) -> Result<u32, String> {
    let mut sb = read_superblock_disk()?;
    let new_id = sb.inode_count;
    
    let inode = Inode {
        id: new_id,
        is_directory: is_dir,
        size: 0,
        owner_uid: uid,
        owner_gid: gid,
        permissions,
        direct_blocks: [0; 6],
    };
    
    write_inode_disk(new_id, &inode)?;
    
    sb.inode_count += 1;
    write_superblock_disk(&sb)?;
    
    Ok(new_id)
}

// Block allocation
pub fn allocate_data_block() -> Result<u32, String> {
    let mut sb = read_superblock_disk()?;
    // For simplicity, find the first block from DATA_START_SECTOR that is not mapped by any inode.
    // In a production kernel we use a free block bitmap. Here we scan inodes for mapping,
    // which is easy, reliable, and perfectly visualizable for students.
    let mut used_blocks = std::collections::HashSet::new();
    for id in 0..sb.inode_count {
        if let Ok(inode) = read_inode_disk(id) {
            for &block in &inode.direct_blocks {
                if block != 0 {
                    used_blocks.insert(block);
                }
            }
        }
    }

    for block in DATA_START_SECTOR..drivers::disk::DISK_SECTORS {
        if !used_blocks.contains(&(block as u32)) {
            sb.free_blocks -= 1;
            write_superblock_disk(&sb)?;
            return Ok(block as u32);
        }
    }
    Err("Out of data blocks".to_string())
}

// NovaFS Write File logic checking Quotas and logging to Journal
pub fn write_file_data(inode_id: u32, offset: usize, data: &[u8], uid: u32) -> Result<(), String> {
    // 1. Quota Enforcement
    let user_limit = {
        let quotas = USER_QUOTAS.lock().unwrap();
        quotas.as_ref().and_then(|q| q.get(&uid).copied())
    };

    if let Some(limit) = user_limit {
        let current_usage = {
            let usages = USER_USAGES.lock().unwrap();
            usages.as_ref().map(|u| u.get(&uid).copied().unwrap_or(0)).unwrap_or(0)
        };
        if current_usage + data.len() as u32 > limit {
            return Err(format!("Disk quota exceeded (Limit: {} bytes, Current: {} bytes)", limit, current_usage));
        }
    }

    // 2. Journal Log entry (for transaction safety)
    write_journal_log(inode_id, "WRITE", offset as u32)?;

    let mut inode = read_inode_disk(inode_id)?;
    if inode.is_directory {
        return Err("Cannot write data to directory".to_string());
    }

    // Calculate how many blocks are needed
    let size_needed = offset + data.len();
    let blocks_needed = (size_needed + 511) / 512;
    if blocks_needed > 6 {
        return Err("File size exceeds direct block capacity".to_string());
    }

    // Allocate blocks as needed
    for i in 0..blocks_needed {
        if inode.direct_blocks[i] == 0 {
            inode.direct_blocks[i] = allocate_data_block()?;
        }
    }

    // Write content
    let mut bytes_written = 0;
    while bytes_written < data.len() {
        let curr_offset = offset + bytes_written;
        let block_idx = curr_offset / 512;
        let block_offset = curr_offset % 512;
        let block_sector = inode.direct_blocks[block_idx] as usize;

        let mut block_buf = [0u8; 512];
        let _ = drivers::disk::read_block(block_sector, &mut block_buf);

        let chunk_size = std::cmp::min(data.len() - bytes_written, 512 - block_offset);
        block_buf[block_offset..(block_offset + chunk_size)]
            .copy_from_slice(&data[bytes_written..(bytes_written + chunk_size)]);

        drivers::disk::write_block(block_sector, &block_buf)?;
        bytes_written += chunk_size;
    }

    inode.size = std::cmp::max(inode.size, size_needed as u32);
    write_inode_disk(inode_id, &inode)?;

    // 3. Update User usage stats
    if let Some(ref mut usages) = *USER_USAGES.lock().unwrap() {
        let usage = usages.entry(uid).or_insert(0);
        *usage += data.len() as u32;
    }

    // 4. Commit Journal transaction
    commit_journal()?;

    Ok(())
}

pub fn read_file_data(inode_id: u32, offset: usize, buf: &mut [u8]) -> Result<usize, String> {
    let inode = read_inode_disk(inode_id)?;
    if inode.is_directory {
        return Err("Cannot read data from a directory".to_string());
    }

    if offset >= inode.size as usize {
        return Ok(0);
    }

    let end = std::cmp::min(inode.size as usize, offset + buf.len());
    let size_to_read = end - offset;
    let mut bytes_read = 0;

    while bytes_read < size_to_read {
        let curr_offset = offset + bytes_read;
        let block_idx = curr_offset / 512;
        let block_offset = curr_offset % 512;
        let block_sector = inode.direct_blocks[block_idx] as usize;

        if block_sector == 0 {
            break; // Sparse block
        }

        let mut block_buf = [0u8; 512];
        drivers::disk::read_block(block_sector, &mut block_buf)?;

        let chunk_size = std::cmp::min(size_to_read - bytes_read, 512 - block_offset);
        buf[bytes_read..(bytes_read + chunk_size)]
            .copy_from_slice(&block_buf[block_offset..(block_offset + chunk_size)]);

        bytes_read += chunk_size;
    }

    Ok(bytes_read)
}

// Journaling Support
fn write_journal_log(inode_id: u32, op: &str, offset: u32) -> Result<(), String> {
    let mut sb = read_superblock_disk()?;
    let tx_id = sb.journal_ptr;
    
    let entry = JournalEntry {
        transaction_id: tx_id,
        inode_id,
        operation: op.to_string(),
        data_offset: offset,
        committed: false,
    };

    let target_sector = JOURNAL_START_SECTOR + (tx_id as usize % JOURNAL_SECTORS);
    let bytes = serde_json::to_vec(&entry).map_err(|e| e.to_string())?;
    let mut sector = [0u8; 512];
    sector[0..bytes.len()].copy_from_slice(&bytes);
    
    drivers::disk::write_block(target_sector, &sector)?;

    sb.journal_ptr += 1;
    write_superblock_disk(&sb)?;
    Ok(())
}

fn commit_journal() -> Result<(), String> {
    let sb = read_superblock_disk()?;
    let last_tx_id = if sb.journal_ptr > 0 { sb.journal_ptr - 1 } else { 0 };
    let target_sector = JOURNAL_START_SECTOR + (last_tx_id as usize % JOURNAL_SECTORS);

    let mut sector = [0u8; 512];
    drivers::disk::read_block(target_sector, &mut sector)?;

    let end = sector.iter().position(|&x| x == 0).unwrap_or(512);
    if end > 0 {
        let mut entry: JournalEntry = serde_json::from_slice(&sector[0..end]).map_err(|e| e.to_string())?;
        entry.committed = true;
        
        let bytes = serde_json::to_vec(&entry).map_err(|e| e.to_string())?;
        let mut new_sector = [0u8; 512];
        new_sector[0..bytes.len()].copy_from_slice(&bytes);
        drivers::disk::write_block(target_sector, &new_sector)?;
    }
    Ok(())
}

// Journal recovery during OS Boot
pub fn recover_journal() -> Result<(), String> {
    let _ = read_superblock_disk()?;
    for i in 0..JOURNAL_SECTORS {
        let sector_id = JOURNAL_START_SECTOR + i;
        let mut sector = [0u8; 512];
        drivers::disk::read_block(sector_id, &mut sector)?;
        let end = sector.iter().position(|&x| x == 0).unwrap_or(512);
        if end > 0 {
            if let Ok(entry) = serde_json::from_slice::<JournalEntry>(&sector[0..end]) {
                if !entry.committed {
                    // Recover: Revert transaction or clean up uncommitted records
                    // For educational transparency: display log during boot recovery
                    drivers::vga_println!("[NovaFS Journal] Recovering transaction {}, uncommitted operation: {}", entry.transaction_id, entry.operation);
                }
            }
        }
    }
    Ok(())
}

// Snapshots creation
pub fn create_snapshot(snapshot_id: u32) -> Result<(), String> {
    let root_inode = read_inode_disk(0)?;
    
    // Perform block-level storage backup of all current data blocks
    let mut backup = HashMap::new();
    for sector in 0..drivers::disk::DISK_SECTORS {
        let mut buffer = [0u8; 512];
        if let Ok(_) = drivers::disk::read_block(sector, &mut buffer) {
            backup.insert(sector, buffer.to_vec());
        }
    }

    let snapshot = Snapshot {
        id: snapshot_id,
        timestamp: 12345678, // simulated timestamp
        root_inode_copy: root_inode,
        disk_backup: backup,
    };

    let mut snaps = SNAPSHOTS.lock().unwrap();
    if let Some(ref mut map) = *snaps {
        map.insert(snapshot_id, snapshot);
    }

    Ok(())
}

// Restoring state from a snapshot
pub fn restore_snapshot(snapshot_id: u32) -> Result<(), String> {
    let snaps = SNAPSHOTS.lock().unwrap();
    if let Some(ref map) = *snaps {
        if let Some(snapshot) = map.get(&snapshot_id) {
            // Restore disk blocks
            for (&sector, data) in &snapshot.disk_backup {
                drivers::disk::write_block(sector, data)?;
            }
            Ok(())
        } else {
            Err("Snapshot ID not found".to_string())
        }
    } else {
        Err("Snapshots table uninitialized".to_string())
    }
}

// Directory operations
pub fn add_dir_entry(dir_inode_id: u32, name: &str, child_inode_id: u32) -> Result<(), String> {
    let mut dir_inode = read_inode_disk(dir_inode_id)?;
    if !dir_inode.is_directory {
        return Err("Parent is not a directory".to_string());
    }

    // Read current entries
    let mut entries = read_directory_entries(dir_inode_id)?;
    entries.push(DirectoryEntry {
        name: name.to_string(),
        inode_id: child_inode_id,
    });

    let bytes = serde_json::to_vec(&entries).map_err(|e| e.to_string())?;
    
    // Allocate space and write
    if dir_inode.direct_blocks[0] == 0 {
        dir_inode.direct_blocks[0] = allocate_data_block()?;
    }
    
    let sector = dir_inode.direct_blocks[0] as usize;
    let mut block_buf = [0u8; 512];
    if bytes.len() > 512 {
        return Err("Directory lists too long for demo disk block limit".to_string());
    }
    block_buf[0..bytes.len()].copy_from_slice(&bytes);
    drivers::disk::write_block(sector, &block_buf)?;

    dir_inode.size = bytes.len() as u32;
    write_inode_disk(dir_inode_id, &dir_inode)?;
    Ok(())
}

pub fn read_directory_entries(dir_inode_id: u32) -> Result<Vec<DirectoryEntry>, String> {
    let dir_inode = read_inode_disk(dir_inode_id)?;
    if !dir_inode.is_directory {
        return Err("Node is not a directory".to_string());
    }

    let sector = dir_inode.direct_blocks[0] as usize;
    if sector == 0 {
        return Ok(Vec::new());
    }

    let mut block_buf = [0u8; 512];
    drivers::disk::read_block(sector, &mut block_buf)?;

    let end = block_buf.iter().position(|&x| x == 0).unwrap_or(512);
    if end == 0 {
        return Ok(Vec::new());
    }

    let entries: Vec<DirectoryEntry> = serde_json::from_slice(&block_buf[0..end]).map_err(|e| e.to_string())?;
    Ok(entries)
}

// Check if a path can be read/written by a specific UID
pub fn check_permission(inode_id: u32, uid: u32, write_req: bool) -> Result<bool, String> {
    // Root can do anything
    if uid == 0 {
        return Ok(true);
    }
    let inode = read_inode_disk(inode_id)?;
    let is_owner = inode.owner_uid == uid;
    
    // Mask details:
    // owner bits: (permissions >> 6) & 0x7
    // group bits: (permissions >> 3) & 0x7
    // other bits: permissions & 0x7
    let perm_bits = if is_owner {
        (inode.permissions >> 6) & 0x7
    } else {
        inode.permissions & 0x7 // fall back to others for demo
    };

    let readable = (perm_bits & 4) != 0;
    let writeable = (perm_bits & 2) != 0;

    if write_req {
        Ok(writeable)
    } else {
        Ok(readable)
    }
}
