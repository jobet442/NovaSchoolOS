# NovaOS - Architecture Design

NovaOS is designed with a hybrid microkernel-inspired Unix-like architecture. The system exposes modular abstraction boundaries so students can study kernel internals in isolation.

```mermaid
graph TD
    subgraph Userspace ["Userspace Environment"]
        shell[NovaShell]
        pkg[NovaPkg]
        utils[Core Utils: ls, cat, ps, top]
    end

    subgraph SystemCall ["System Call Interface"]
        sys_fork[sys_fork]
        sys_write[sys_write]
        sys_read[sys_read]
        sys_socket[sys_socket]
        sys_kill[sys_kill]
    end

    subgraph Kernel ["NovaOS Hybrid Kernel"]
        direction TB
        scheduler[Preemptive CPU Scheduler<br>Round-Robin | Priority | RT]
        mem[Virtual Memory Manager<br>Page Tables | COW | Frame Allocator]
        vfs[Virtual Filesystem VFS<br>NovaFS | FAT32 | EXT2]
        net[TCP/IP Network Stack<br>IPv4 | TCP | UDP | SSH Daemon]
        sec[Security Framework<br>MAC | Capability Checks | Audits]
        interrupts[Interrupt Descriptor Table IDT<br>Exceptions | Keyboard | Timer]
    end

    subgraph Hardware ["Hardware / Drivers Abstraction"]
        vga[VGA Framebuffer Driver]
        kbd[Keyboard/Mouse Driver]
        disk[Block Storage IDE Driver]
        nic[Ethernet Network Driver]
    end

    %% Flow lines
    shell -->|POSIX System Calls| SystemCall
    utils -->|POSIX System Calls| SystemCall
    SystemCall -->|Trap Dispatcher| Kernel
    
    %% Internal Kernel interactions
    vfs -->|Block IO| disk
    net -->|Packet Frame Transmission| nic
    interrupts -->|ISR Events| scheduler
    mem -->|Page faults vector 14| interrupts
    sec -->|Mandatory Access Checks| vfs
    
    %% Output visualization
    Kernel -->|VGA print stream| vga
    kbd -->|IRQ 1 interrupts| interrupts
```

## System Subsystems Detail

1. **Virtual Filesystem (VFS) & NovaFS**:
   Exposes POSIX-like file descriptor mappings. NovaFS utilizes simple sector-based structures, allocating 512-byte blocks. Transaction records are written to a scrolling journal zone before write execution and marked committed upon block write completion.
   
2. **Paging & Copy-On-Write (COW)**:
   Supports virtual page mapping. Forking clones page tables, disabling the write-flag on parent and child page table entries. Attempts to write to these pages trigger a CPU page fault (Vector 14 exception), where the kernel allocates a new physical frame, duplicates the block contents, and marks the page writable.

3. **Preemptive Task Scheduling**:
   Interrupt service routines triggered by timer tick events preempt active processes. The scheduler supports swapping between Round-Robin cycle scheduling, priority weight selections, and strict Real-Time preemption windows.
