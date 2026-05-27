# NovaOS - Student Developer Handbook

Welcome, student! This developer handbook provides guides and interactive lab exercises to study operating system internals on NovaOS.

---

## Lab 1: Explaining CPU Scheduling

In this lab, we will witness how processes compete for CPU run ticks and how scheduling policies affect multitasking execution.

### Steps:
1. Boot NovaOS by running the simulator binary.
2. Select the **Student Terminal** tab (F1) and run:
   ```bash
   ps
   ```
   Note the list of active processes (`idle`, `kswapd`, `sshd`, and your shell).
3. The scheduler switches tasks automatically in the background. Press F1/F2 keys and check the **Process PCB Table** in the kernel visualizer panel.
4. Watch how ticks accumulate under the `CPU_TICKS` column.
5. Ask **Nova Assistant** (F4 Tab) to explain scheduling:
   *Query:* `scheduler` or `explain scheduling`

---

## Lab 2: Copy-on-Write and Page Fault Exceptions

This lab demonstrates how the kernel optimizes memory usage when copying processes (forking).

### Background:
When `sys_fork` is called, the kernel doesn't immediately duplicate all RAM blocks. It points both processes' page tables to the same physical frames but sets pages as **Read-Only**. Only when one process attempts to write to a page, a **Page Fault Exception** is raised. The kernel copies the page frame on demand.

### Steps:
1. In the terminal (F1), type:
   ```bash
   cat /assignments/lab1_instructions.txt > /students/student1001/work.txt
   ```
2. Check the **Security Auditor** tab (F5) to see the audit event logging the MAC (Mandatory Access Control) validation.
3. Observe the **System Interrupt Counters** panel in the kernel visualizer. Every page write during simulation increases the **Page Fault Exceptions** count.
4. Read the details of how the Page Fault handler allocates a new physical frame number and re-maps it to the active process.

---

## Lab 3: Filesystem Journaling Recovery

In this lab, we will check how the transaction journal prevents corruption on crashes.

### Steps:
1. In the terminal, create a snapshot of your clean environment:
   ```bash
   snapshot create 1
   ```
2. Write a file using redirection:
   ```bash
   echo "Task completion log" > /students/student1001/results.txt
   ```
3. The transaction writes to the **Journal Area** (sectors 11-15) before executing block writes.
4. Revert back to snapshot 1 to watch the VFS wipe recent changes and restore initial metadata:
   ```bash
   snapshot restore 1
   ```
5. Run `ls` to verify the state of your students workspace directory.
