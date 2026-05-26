pub struct Ext2Simulator {
    pub uuid: String,
}

impl Ext2Simulator {
    pub fn new() -> Self {
        Ext2Simulator {
            uuid: "3b2512f4-7e88-4221-a1b7-e231123ad82d".to_string(),
        }
    }

    pub fn list_inodes(&self) -> Vec<(u32, String, u16)> {
        vec![
            (2, ".".to_string(), 0o755),
            (11, "lost+found".to_string(), 0o700),
            (12, "notes.txt".to_string(), 0o644),
            (13, "lab_config".to_string(), 0o755),
        ]
    }

    pub fn read_file_by_inode(&self, inode: u32) -> Result<String, String> {
        match inode {
            12 => Ok("EXT2 File System notes:\nStandard Linux layout simulation.\nInode 2 is root.\nInode 11 is lost+found.\n".to_string()),
            13 => Ok("# Classroom Lab Config\nASSIGNMENT_ID=lab1\nDEADLINE=2026-06-01\n".to_string()),
            _ => Err("Invalid inode or inode is a directory".to_string()),
        }
    }
}
