use std::sync::Mutex;

pub const SECTOR_SIZE: usize = 512;
pub const DISK_SECTORS: usize = 4096; // 2MB Virtual Disk for educational light footprint
pub const DISK_SIZE: usize = DISK_SECTORS * SECTOR_SIZE;

pub struct DiskDevice {
    storage: Vec<u8>,
}

impl DiskDevice {
    pub fn new() -> Self {
        DiskDevice {
            storage: vec![0; DISK_SIZE],
        }
    }

    pub fn read_sector(&self, sector: usize, buf: &mut [u8]) -> Result<(), String> {
        if sector >= DISK_SECTORS {
            return Err("Sector out of bounds".to_string());
        }
        if buf.len() < SECTOR_SIZE {
            return Err("Buffer too small".to_string());
        }
        let start = sector * SECTOR_SIZE;
        buf[0..SECTOR_SIZE].copy_from_slice(&self.storage[start..(start + SECTOR_SIZE)]);
        Ok(())
    }

    pub fn write_sector(&mut self, sector: usize, buf: &[u8]) -> Result<(), String> {
        if sector >= DISK_SECTORS {
            return Err("Sector out of bounds".to_string());
        }
        if buf.len() < SECTOR_SIZE {
            return Err("Buffer too small".to_string());
        }
        let start = sector * SECTOR_SIZE;
        self.storage[start..(start + SECTOR_SIZE)].copy_from_slice(&buf[0..SECTOR_SIZE]);
        Ok(())
    }

    pub fn size_sectors(&self) -> usize {
        DISK_SECTORS
    }
}

// Thread-safe global disk device
static DISK_DEVICE: Mutex<Option<DiskDevice>> = Mutex::new(None);

pub fn init_disk() {
    let disk = DiskDevice::new();
    // We could pre-initialize the disk with a raw partition table or format it later.
    // For educational purposes, starting with zeroed sectors is standard.
    *DISK_DEVICE.lock().unwrap() = Some(disk);
}

pub fn read_block(sector: usize, buf: &mut [u8]) -> Result<(), String> {
    let lock = DISK_DEVICE.lock().unwrap();
    if let Some(ref disk) = *lock {
        disk.read_sector(sector, buf)
    } else {
        Err("Disk not initialized".to_string())
    }
}

pub fn write_block(sector: usize, buf: &[u8]) -> Result<(), String> {
    let mut lock = DISK_DEVICE.lock().unwrap();
    if let Some(ref mut disk) = *lock {
        disk.write_sector(sector, buf)
    } else {
        Err("Disk not initialized".to_string())
    }
}
