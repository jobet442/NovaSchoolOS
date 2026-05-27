pub struct Fat32Simulator {
    pub label: String,
}

impl Fat32Simulator {
    pub fn new() -> Self {
        Fat32Simulator {
            label: "NOVA_BOOT".to_string(),
        }
    }

    pub fn list_files(&self) -> Vec<(String, usize)> {
        vec![
            ("EFI/BOOT/BOOTX64.EFI".to_string(), 45056),
            ("KERNEL.BIN".to_string(), 262144),
            ("SYSTEM.TXT".to_string(), 124),
        ]
    }

    pub fn read_file(&self, path: &str) -> Result<String, String> {
        match path {
            "SYSTEM.TXT" => Ok("NovaOS FAT32 Boot Recovery Partition\nStatus: OK\nVersion: 0.1.0\n".to_string()),
            "EFI/BOOT/BOOTX64.EFI" => Ok("[Binary EFI Payload]".to_string()),
            "KERNEL.BIN" => Ok("[Kernel executable payload]".to_string()),
            _ => Err("File not found on FAT32 partition".to_string()),
        }
    }
}
