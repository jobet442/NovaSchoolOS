# NovaSchool OS - System Call Reference

System calls bridge user space applications and the privileged kernel mode. In NovaSchool OS, system calls are intercepted via an Interrupt Descriptor Table vector trap or a software trigger.

## Syscall Table

| ID | Syscall Name | Arguments | Return Value | Description |
|---|---|---|---|---|
| `1` | `sys_exit` | `arg1`: Exit code | `0` | Terminates the calling process and releases physical memory frame allocations. |
| `2` | `sys_fork` | None | Child `PID` | Duplicates the active process address space, utilizing Copy-On-Write page mappings. |
| `3` | `sys_read` | `arg1`: File Descriptor<br>`arg2`: Buffer Address<br>`arg3`: Length | Number of bytes read | Reads bytes from active file descriptor into a buffer. |
| `4` | `sys_write` | `arg1`: File Descriptor<br>`arg2`: Buffer Address<br>`arg3`: Length | Number of bytes written | Writes bytes from a buffer into a file descriptor. Enforces MAC write restrictions. |
| `5` | `sys_yield` | None | `0` | Voluntary thread context yields, relinquishing remaining CPU ticks back to scheduler queues. |
| `6` | `sys_kill` | `arg1`: Target Process PID | `0` | Terminates a running process. Requires `Capability::ProcessKill` permissions. |
| `7` | `sys_socket` | `arg1`: Bind Network Port | `0` | Binds a virtual TCP network socket onto a designated port listener. |

## Educational Experiments for Students

1. **Syscall Logger Tab**:
   Open the **Syscall Explorer** tab (F2) in the dashboard, execute shell commands (e.g. `cat file.txt` or `ls`), and watch the scroll list populate. You will trace:
   - `sys_read` triggers on file access.
   - `sys_write` triggers on writing directories or stdout streams.
   
2. **Security Violations**:
   Try running `novapkg install gcc` when logged in as a student. The package manager opens network ports, triggering `sys_socket`. The kernel security auditor logs a capability check event (`CAP_NET_RAW` verification) to the auditor screen!
