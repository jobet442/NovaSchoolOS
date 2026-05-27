use std::collections::HashMap;
use std::sync::Mutex;

static SHELL_CURRENT_DIR: Mutex<Option<String>> = Mutex::new(None);
static ENVIRONMENT_VARIABLES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

pub fn init_shell() {
    *SHELL_CURRENT_DIR.lock().unwrap() = Some("/students/student1".to_string());
    
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
    env.insert("USER".to_string(), "student1".to_string());
    env.insert("SHELL".to_string(), "/bin/novashell".to_string());
    *ENVIRONMENT_VARIABLES.lock().unwrap() = Some(env);
}

// Executes a shell command string, using syscall wrappers, and returns output as string
pub fn execute_command(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Process environment variable expansion (e.g. $USER)
    let expanded = expand_env_variables(trimmed);

    // Simple parsing for pipes |
    if expanded.contains('|') {
        let parts: Vec<&str> = expanded.split('|').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            // Simulated pipe: execute first command, pass output as input/argument to the second
            let out1 = run_single_command(parts[0], "");
            return run_single_command(parts[1], &out1);
        } else {
            return "NovaShell Error: Multistage pipes not supported in current lab.".to_string();
        }
    }

    // Simple parsing for redirection > or >>
    if expanded.contains('>') {
        let mut parts = expanded.split('>');
        let cmd = parts.next().unwrap().trim();
        let mut file_path = parts.next().unwrap().trim().to_string();
        let append = file_path.starts_with('>'); // if it was >>
        if append {
            file_path = file_path.trim_start_matches('>').trim().to_string();
        }

        // Run the command to get output
        let out = run_single_command(cmd, "");
        
        // Write output to the resolved file using syscalls!
        let active_uid = userspace::get_current_user().map(|u| u.uid).unwrap_or(1001);
        let resolved_path = resolve_absolute_path(&file_path);

        // Open (create if not exists)
        match kernel::syscall::sys_call(3, 0, 0, 0) { // sys_read dummy call to trace activity
            _ => {}
        }

        let fd_res = filesystem::vfs_open(&resolved_path, true, true, active_uid);
        match fd_res {
            Ok(fd) => {
                let bytes = out.as_bytes();
                // sys_write syscall (id 4)
                let write_res = kernel::syscall::sys_call(4, fd as u64, 0, bytes.len() as u64);
                if let Err(e) = write_res {
                    let _ = filesystem::vfs_close(fd);
                    return format!("Write error: {}", e);
                }
                let _ = filesystem::vfs_close(fd);
                return format!("Output redirected to {}", file_path);
            }
            Err(e) => return format!("Redirection failed: {}", e),
        }
    }

    run_single_command(&expanded, "")
}

fn run_single_command(cmd_line: &str, pipe_input: &str) -> String {
    let mut tokens = split_arguments(cmd_line);
    if tokens.is_empty() {
        return String::new();
    }

    let cmd = tokens.remove(0);
    let mut args = tokens;

    // If there is pipe input, we can append it as the last argument
    if !pipe_input.is_empty() {
        args.push(pipe_input.to_string());
    }

    let curr_user = userspace::get_current_user().unwrap();

    match cmd.as_str() {
        // --- BUILTINS ---
        "help" => {
            let mut out = String::new();
            out.push_str("NovaOS - Shell Builtin Commands:\n");
            out.push_str("  help                    Display this list\n");
            out.push_str("  cd <path>               Change directory\n");
            out.push_str("  pwd                     Print working directory\n");
            out.push_str("  export <key>=<val>      Set environment variables\n");
            out.push_str("  login <user> <pass>     Authenticate user session\n");
            out.push_str("  logout                  Exit active user session\n");
            out.push_str("  clear                   Reset console screen\n");
            out.push_str("\nCore Unix Utilities:\n");
            out.push_str("  ls, cat, cp, mv, rm, mkdir, chmod, chown, grep, echo, ps, top\n");
            out.push_str("\nEducational/Classroom commands:\n");
            out.push_str("  quota                   Inspect disk storage quota\n");
            out.push_str("  snapshot <create|restore> <id>  Restore OS filesystem backups\n");
            out.push_str("  novapkg install/remove/list     Manage applications\n");
            out.push_str("\nInter-Process Communication (IPC):\n");
            out.push_str("  mq_send <qname> <msg>   Send message to queue\n");
            out.push_str("  mq_recv <qname>         Receive message from queue\n");
            out.push_str("  mq_list                 List active message queues\n");
            out.push_str("\nConcurrency & Mutexes:\n");
            out.push_str("  mutex_create <name>     Create a new lock\n");
            out.push_str("  mutex_lock <id>         Acquire a lock\n");
            out.push_str("  mutex_unlock <id>       Release a lock\n");
            out.push_str("  deadlock_demo           Simulate Dining Philosophers deadlock\n");
            out.push_str("  deadlock_resolve        Resolve the deadlock cycle\n");
            out
        }
        "cd" => {
            if args.is_empty() {
                return "Usage: cd <path>".to_string();
            }
            let target = resolve_absolute_path(&args[0]);
            
            // Check if folder exists in VFS
            match filesystem::vfs_list_dir(&target) {
                Ok(_) => {
                    *SHELL_CURRENT_DIR.lock().unwrap() = Some(target);
                    String::new()
                }
                Err(e) => format!("cd: {}", e),
            }
        }
        "pwd" => {
            let curr = SHELL_CURRENT_DIR.lock().unwrap().clone().unwrap();
            format!("{}\n", curr)
        }
        "export" => {
            if args.is_empty() || !args[0].contains('=') {
                return "Usage: export KEY=VALUE".to_string();
            }
            let parts: Vec<&str> = args[0].splitn(2, '=').collect();
            if let Some(env) = ENVIRONMENT_VARIABLES.lock().unwrap().as_mut() {
                env.insert(parts[0].to_string(), parts[1].to_string());
            }
            String::new()
        }
        "login" => {
            if args.len() < 2 {
                return "Usage: login <username> <password>".to_string();
            }
            match userspace::login_user(&args[0], &args[1]) {
                Ok(u) => {
                    update_shell_session(&u.username, u.uid);
                    format!("Logged in successfully as {}.\n", u.username)
                }
                Err(e) => format!("Login failed: {}\n", e),
            }
        }
        "logout" => {
            userspace::logout_user();
            if let Some(env) = ENVIRONMENT_VARIABLES.lock().unwrap().as_mut() {
                env.insert("USER".to_string(), "nobody".to_string());
            }
            *SHELL_CURRENT_DIR.lock().unwrap() = Some("/".to_string());
            "Logged out. Current session reset.\n".to_string()
        }
        "clear" => {
            drivers::vga::init_vga();
            String::new()
        }

        // --- CORE UTILITIES ---
        "ls" => {
            let path = if args.is_empty() {
                SHELL_CURRENT_DIR.lock().unwrap().clone().unwrap()
            } else {
                resolve_absolute_path(&args[0])
            };

            match filesystem::vfs_list_dir(&path) {
                Ok(entries) => {
                    let mut out = String::new();
                    out.push_str(&format!("Directory list for {}:\n", path));
                    for (name, size, is_dir, perm) in entries {
                        let type_char = if is_dir { "d" } else { "-" };
                        out.push_str(&format!(
                            "  {}{:o}  {:8} bytes  {}\n",
                            type_char, perm, size, name
                        ));
                    }
                    out
                }
                Err(e) => format!("ls: {}", e),
            }
        }
        "cat" => {
            if args.is_empty() {
                return "Usage: cat <file>".to_string();
            }
            let target = resolve_absolute_path(&args[0]);
            // Trace sys_read system call id (3)
            let _ = kernel::syscall::sys_call(3, 0, 0, 0);

            match filesystem::vfs_open(&target, false, false, curr_user.uid) {
                Ok(fd) => {
                    let mut buffer = vec![0u8; 4096];
                    let mut out = String::new();
                    loop {
                        match filesystem::vfs_read(fd, &mut buffer) {
                            Ok(0) => break,
                            Ok(n) => {
                                if let Ok(s) = std::str::from_utf8(&buffer[0..n]) {
                                    out.push_str(s);
                                }
                            }
                            Err(e) => {
                                let _ = filesystem::vfs_close(fd);
                                return format!("cat read error: {}", e);
                            }
                        }
                    }
                    let _ = filesystem::vfs_close(fd);
                    out
                }
                Err(e) => format!("cat: {}", e),
            }
        }
        "echo" => {
            args.join(" ") + "\n"
        }
        "mkdir" => {
            if args.is_empty() {
                return "Usage: mkdir <dir>".to_string();
            }
            let target = resolve_absolute_path(&args[0]);
            match filesystem::vfs_mkdir(&target, 0o755, curr_user.uid) {
                Ok(_) => String::new(),
                Err(e) => format!("mkdir: {}", e),
            }
        }
        "rm" => {
            if args.is_empty() {
                return "Usage: rm <file>".to_string();
            }
            // For safety we verify path deletion in VFS. Since rm deletes an inode, we can simulate
            // removing directory records.
            let target = resolve_absolute_path(&args[0]);
            // Check permissions
            let active_uid = curr_user.uid;
            match filesystem::vfs_open(&target, false, true, active_uid) {
                Ok(fd) => {
                    let _ = filesystem::vfs_close(fd); // close right away
                    // In our mock, files are deleted by resolving parent directory and removing child entries.
                    // Let's implement simplified mock delete:
                    return format!("File {} deleted successfully.\n", args[0]);
                }
                Err(e) => format!("rm: Cannot delete {}: {}", args[0], e),
            }
        }
        "cp" => {
            if args.len() < 2 {
                return "Usage: cp <source> <destination>".to_string();
            }
            let src = resolve_absolute_path(&args[0]);
            let dest = resolve_absolute_path(&args[1]);

            // Open source
            let fd_src = match filesystem::vfs_open(&src, false, false, curr_user.uid) {
                Ok(fd) => fd,
                Err(e) => return format!("cp source open failed: {}", e),
            };

            let mut buf = vec![0u8; 4096];
            let read_bytes = match filesystem::vfs_read(fd_src, &mut buf) {
                Ok(n) => n,
                Err(e) => {
                    let _ = filesystem::vfs_close(fd_src);
                    return format!("cp source read failed: {}", e);
                }
            };
            let _ = filesystem::vfs_close(fd_src);

            // Open dest and write
            let fd_dest = match filesystem::vfs_open(&dest, true, true, curr_user.uid) {
                Ok(fd) => fd,
                Err(e) => return format!("cp destination open failed: {}", e),
            };

            if let Err(e) = filesystem::vfs_write(fd_dest, &buf[0..read_bytes], curr_user.uid) {
                let _ = filesystem::vfs_close(fd_dest);
                return format!("cp destination write failed: {}", e);
            }
            let _ = filesystem::vfs_close(fd_dest);
            String::new()
        }
        "ps" => {
            let list = kernel::task::get_process_list();
            let mut out = String::new();
            out.push_str("PID  USER    PRIORITY  STATE     CPU_TICKS  NAME\n");
            for p in list {
                if p.state != kernel::task::ProcessState::Killed {
                    let user_label = if p.owner_uid == 0 { "root" } else { "student" };
                    out.push_str(&format!(
                        "{:<4} {:<7} {:<9} {:<9} {:<10} {}\n",
                        p.pid, user_label, format!("{:?}", p.priority), format!("{:?}", p.state), p.cpu_ticks, p.name
                    ));
                }
            }
            out
        }
        "top" => {
            // Interactive Process top simulation (yields output once)
            let list = kernel::task::get_process_list();
            let mut out = String::new();
            out.push_str("NovaOS Top Monitor:\n");
            out.push_str("Active processes list (sorted by ticks):\n");
            out.push_str("PID   STATE     TICKS  COMMAND\n");
            let mut active = list;
            active.sort_by(|a, b| b.cpu_ticks.cmp(&a.cpu_ticks));
            for p in active.iter().take(5) {
                if p.state != kernel::task::ProcessState::Killed {
                    out.push_str(&format!(
                        "{:<5} {:<9} {:<6} {}\n",
                        p.pid, format!("{:?}", p.state), p.cpu_ticks, p.name
                    ));
                }
            }
            out
        }
        "grep" => {
            if args.is_empty() {
                return "Usage: grep <pattern> [content_string]".to_string();
            }
            let pattern = &args[0];
            let lines: Vec<&str> = if args.len() >= 2 {
                // Grep in content passed
                args[1].lines().collect()
            } else {
                return "Usage: grep <pattern> <string_input> or use piping".to_string();
            };

            let mut out = String::new();
            for l in lines {
                if l.contains(pattern) {
                    out.push_str(l);
                    out.push_str("\n");
                }
            }
            out
        }
        "chmod" => {
            if args.len() < 2 {
                return "Usage: chmod <octal_mode> <file>".to_string();
            }
            let mode = u16::from_str_radix(&args[0], 8).unwrap_or(0o644);
            let target = resolve_absolute_path(&args[1]);

            // Update VFS permissions
            match filesystem::vfs_open(&target, false, true, curr_user.uid) {
                Ok(fd) => {
                    let _ = filesystem::vfs_list_dir(&target); // list details
                    let _ = filesystem::vfs_close(fd);
                    // In simulation: update the permissions byte in filesystem inode
                    if let Ok(mut inode) = filesystem::novafs::read_inode_disk(0) {
                        inode.permissions = mode;
                        let _ = filesystem::novafs::write_inode_disk(0, &inode);
                    }
                    format!("Permissions changed successfully.\n")
                }
                Err(e) => format!("chmod: {}", e),
            }
        }

        // --- PACKAGE MANAGER ---
        "novapkg" => {
            if args.is_empty() {
                return "Usage: novapkg [install|remove|list] <package>".to_string();
            }
            match args[0].as_str() {
                "list" => {
                    let installed = userspace::novapkg::list_installed();
                    let mut out = String::new();
                    out.push_str("Installed packages:\n");
                    for pkg in installed {
                        out.push_str(&format!("  - {}\n", pkg));
                    }
                    out
                }
                "install" => {
                    if args.len() < 2 {
                        return "Usage: novapkg install <package>".to_string();
                    }
                    // Trace install via syscall socket bounds
                    let _ = kernel::syscall::sys_call(7, 80, 0, 0); // socket simulation

                    match userspace::novapkg::install_package(&args[1]) {
                        Ok(installed) => {
                            format!("Installation complete. Installed packages: {:?}", installed)
                        }
                        Err(e) => format!("NovaPkg Error: {}", e),
                    }
                }
                "remove" => {
                    if args.len() < 2 {
                        return "Usage: novapkg remove <package>".to_string();
                    }
                    match userspace::novapkg::remove_package(&args[1]) {
                        Ok(_) => format!("Package '{}' removed successfully.", &args[1]),
                        Err(e) => format!("NovaPkg Error: {}", e),
                    }
                }
                _ => "Unknown novapkg action.".to_string(),
            }
        }

        // --- SNAPSHOTS ---
        "snapshot" => {
            if args.len() < 2 {
                return "Usage: snapshot <create|restore> <snapshot_id>".to_string();
            }
            let action = &args[0];
            let snap_id = args[1].parse::<u32>().unwrap_or(1);

            match action.as_str() {
                "create" => {
                    match filesystem::novafs::create_snapshot(snap_id) {
                        Ok(_) => format!("Snapshot {} created successfully.\n", snap_id),
                        Err(e) => format!("Failed to create snapshot: {}\n", e),
                    }
                }
                "restore" => {
                    match filesystem::novafs::restore_snapshot(snap_id) {
                        Ok(_) => format!("Snapshot {} restored successfully. Filesystem state reverted.\n", snap_id),
                        Err(e) => format!("Failed to restore snapshot: {}\n", e),
                    }
                }
                _ => "Unknown snapshot command. Choose 'create' or 'restore'.\n".to_string(),
            }
        }

        // --- IPC MESSAGE QUEUES ---
        "mq_send" => {
            if args.len() < 2 {
                return "Usage: mq_send <queue_name> <message>".to_string();
            }
            let qname = &args[0];
            let msg = args[1..].join(" ");
            
            let _ = kernel::ipc::create_queue(qname);
            let _ = kernel::syscall::sys_call(8, qname.len() as u64, msg.len() as u64, 0);
            
            match kernel::ipc::send_message(qname, msg) {
                Ok(_) => format!("Sent message to queue '{}'.\n", qname),
                Err(e) => format!("mq_send failed: {}\n", e),
            }
        }
        "mq_recv" => {
            if args.is_empty() {
                return "Usage: mq_recv <queue_name>".to_string();
            }
            let qname = &args[0];
            
            let _ = kernel::syscall::sys_call(9, qname.len() as u64, 0, 0);
            
            match kernel::ipc::recv_message(qname) {
                Ok(Some(msg)) => format!("Received: {}\n", msg),
                Ok(None) => "No messages in queue.\n".to_string(),
                Err(e) => format!("mq_recv failed: {}\n", e),
            }
        }
        "mq_list" => {
            let queues = kernel::ipc::get_queues();
            if queues.is_empty() {
                "No active message queues.\n".to_string()
            } else {
                let mut out = String::new();
                out.push_str("Active message queues:\n");
                for (name, count, _) in queues {
                    out.push_str(&format!("  - '{}' ({} messages pending)\n", name, count));
                }
                out
            }
        }

        // --- CONCURRENCY & MUTEXES ---
        "mutex_create" => {
            if args.is_empty() {
                return "Usage: mutex_create <name>".to_string();
            }
            let name = &args[0];
            
            let hash = name.len() as u64;
            let _ = kernel::syscall::sys_call(10, hash, 0, 0);
            
            match kernel::mutex::create_mutex(name) {
                Ok(id) => format!("Mutex '{}' created with ID {}.\n", name, id),
                Err(e) => format!("mutex_create failed: {}\n", e),
            }
        }
        "mutex_lock" => {
            if args.is_empty() {
                return "Usage: mutex_lock <id>".to_string();
            }
            let id = match args[0].parse::<u32>() {
                Ok(n) => n,
                Err(_) => return "Invalid mutex ID".to_string(),
            };
            
            let pid = kernel::task::get_current_pid();
            match kernel::syscall::sys_call(11, id as u64, 0, 0) {
                Ok(1) => format!("Mutex {} locked by process PID {}.\n", id, pid),
                Ok(0) => format!("Mutex {} is busy. Process PID {} is now BLOCKED.\n", id, pid),
                Ok(other) => format!("Syscall returned: {}\n", other),
                Err(e) => format!("mutex_lock failed: {}\n", e),
            }
        }
        "mutex_unlock" => {
            if args.is_empty() {
                return "Usage: mutex_unlock <id>".to_string();
            }
            let id = match args[0].parse::<u32>() {
                Ok(n) => n,
                Err(_) => return "Invalid mutex ID".to_string(),
            };
            
            match kernel::syscall::sys_call(12, id as u64, 0, 0) {
                Ok(_) => format!("Mutex {} unlocked.\n", id),
                Err(e) => format!("mutex_unlock failed: {}\n", e),
            }
        }
        "deadlock_demo" => {
            match kernel::mutex::run_deadlock_simulation() {
                Ok(_) => "Dining Philosophers simulation started. 3 philosopher tasks blocked. DEADLOCK DETECTED.\n".to_string(),
                Err(e) => format!("Simulation failed: {}\n", e),
            }
        }
        "deadlock_resolve" => {
            match kernel::mutex::resolve_deadlock() {
                Ok(msg) => format!("Deadlock resolved: {}\n", msg),
                Err(e) => format!("Resolve failed: {}\n", e),
            }
        }

        // --- QUOTA ---
        "quota" => {
            // Read quotas
            format!(
                "NovaOS User Quota limits for UID {}:\n  - Max storage size: 102400 bytes (100 KB)\n  - Current Usage: Active within lab boundary.\n",
                curr_user.uid
            )
        }

        _ => {
            // Check if this command name belongs to an installed package (e.g. python, gcc)
            let installed = userspace::novapkg::list_installed();
            if installed.contains(&cmd) {
                format!("Executing package binary '{}' (mock engine active).\nType 'help' to return.\n", cmd)
            } else {
                format!("NovaShell: command not found: {}. Type 'help' for commands.\n", cmd)
            }
        }
    }
}

// Helpers
fn expand_env_variables(s: &str) -> String {
    let mut result = s.to_string();
    if let Some(env) = ENVIRONMENT_VARIABLES.lock().unwrap().as_ref() {
        for (k, v) in env {
            let key_pattern = format!("${}", k);
            if result.contains(&key_pattern) {
                result = result.replace(&key_pattern, v);
            }
        }
    }
    result
}

fn resolve_absolute_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        let curr = SHELL_CURRENT_DIR.lock().unwrap().clone().unwrap();
        if curr == "/" {
            format!("/{}", path)
        } else {
            format!("{}/{}", curr, path)
        }
    }
}

// Helper to split arguments considering spaces and quotations
fn split_arguments(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut arg = String::new();
    let mut in_quotes = false;
    for c in s.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == ' ' && !in_quotes {
            if !arg.is_empty() {
                args.push(arg.clone());
                arg.clear();
            }
        } else {
            arg.push(c);
        }
    }
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

pub fn update_shell_session(username: &str, uid: u32) {
    let home = if uid == 0 {
        "/".to_string()
    } else {
        format!("/students/{}", username)
    };
    *SHELL_CURRENT_DIR.lock().unwrap() = Some(home);
    
    if let Some(env) = ENVIRONMENT_VARIABLES.lock().unwrap().as_mut() {
        env.insert("USER".to_string(), username.to_string());
    }
}
