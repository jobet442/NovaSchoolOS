use std::collections::HashMap;

pub struct NovaAssistant {
    knowledge_base: HashMap<String, String>,
}

impl NovaAssistant {
    pub fn new() -> Self {
        let mut kb = HashMap::new();
        
        kb.insert("scheduler".to_string(), 
            "NovaSchool OS uses three scheduler policies:\n\
            1. Round-Robin: Cycles through ready PIDs giving equal slice (ticks).\n\
            2. Priority: Selects process with highest Priority enum (RealTime > High > Normal).\n\
            3. Real-Time: RT processes preempt all normal/low priority tasks instantly.\n\
            Tip: Use the scheduler configuration panel to swap policies and watch ticks run!".to_string());
            
        kb.insert("memory".to_string(), 
            "Virtual memory in NovaSchool OS is structured as a 2-Level page table directory.\n\
            - Page Size: 4096 bytes (4 KB).\n\
            - Physical RAM: 128 MB mapped into 32,768 physical page frames.\n\
            - Page Faults (Vector 14): Triggered on access violations or during Copy-On-Write (COW).\n\
            When a write happens on a shared COW page, the kernel copies the page block dynamically!".to_string());

        kb.insert("paging".to_string(), kb.get("memory").unwrap().clone());
        
        kb.insert("filesystem".to_string(), 
            "NovaFS is our custom structured educational filesystem. Layout:\n\
            - Sector 1: Superblock containing file totals and journal offsets.\n\
            - Sectors 2-10: Inode Table. Inodes track file size, permissions, owner UID/GID, and direct block pointers.\n\
            - Sectors 11-15: Transaction Journal to recover incomplete actions during unexpected panic halts.\n\
            - User Quotas: Student UIDs are limited to 100 KB max file sizes.".to_string());

        kb.insert("novafs".to_string(), kb.get("filesystem").unwrap().clone());
        
        kb.insert("syscall".to_string(), 
            "System Calls (syscalls) bridge userspace and the kernel.\n\
            - sys_fork (2): Clones the caller process address space.\n\
            - sys_read (3) / sys_write (4): Interact with Virtual Filesystem file descriptors.\n\
            - sys_socket (7): Binds network ports for network sockets.\n\
            When executing commands in the shell, watch the Syscall Log panel to inspect the flow in real-time!".to_string());

        kb.insert("help".to_string(), 
            "I can explain operating system concepts. Ask me about:\n\
            - 'scheduler' (Round-robin, priorities)\n\
            - 'memory' or 'paging' (Page tables, frames, COW, page faults)\n\
            - 'novafs' or 'filesystem' (Inodes, journal logs, snapshots, quotas)\n\
            - 'syscall' (POSIX transitions, sys_fork, sys_write)\n\
            - Or ask about commands like 'ls', 'cat', 'grep', 'novapkg'!".to_string());

        kb.insert("ls".to_string(), "ls: Lists directory contents. In VFS, directories can be on NovaFS, FAT32, or EXT2 partitions.".to_string());
        kb.insert("cat".to_string(), "cat: Reads files and writes contents to stdout. Behind the scenes, it invokes sys_read in a loop.".to_string());
        kb.insert("grep".to_string(), "grep: Searches input lines for a specific string pattern. Can be combined with pipes (e.g. ls | grep student).".to_string());
        kb.insert("novapkg".to_string(), "novapkg: Classroom package manager. Employs DFS topological sort to resolve library dependencies automatically.".to_string());

        NovaAssistant { knowledge_base: kb }
    }

    pub fn ask(&self, query: &str) -> String {
        let clean = query.trim().to_lowercase();
        if clean.is_empty() {
            return "How can I assist you today? Type 'help' to see topics.".to_string();
        }

        // Try exact match first
        if let Some(resp) = self.knowledge_base.get(&clean) {
            return resp.clone();
        }

        // Try fuzzy keyword search
        for (key, resp) in &self.knowledge_base {
            if clean.contains(key) {
                return resp.clone();
            }
        }

        "I'm not sure about that concept yet. Try asking about 'scheduler', 'memory', 'filesystem', 'syscall', or 'novapkg'!".to_string()
    }
}
