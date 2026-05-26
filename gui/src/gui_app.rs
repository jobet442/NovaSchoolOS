#![cfg(feature = "gui-window")]

use eframe::egui;
use crate::classroom;
use crate::assistant;
use crate::ASSISTANT_HISTORY;
use crate::SHELL_INPUT_BUFFER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    KernelVisualizer,
    SyscallExplorer,
    TeacherDashboard,
    AiAssistant,
    SecurityAuditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeacherTab {
    ClassroomControl,
    SecurityAuditor,
}

pub struct NovaGuiApp {
    active_tab: Tab,
    assistant_agent: assistant::NovaAssistant,
    announcement_input: String,
    chat_input: String,
    selected_pid: Option<u32>,
    teacher_window_open: bool,
    teacher_active_tab: TeacherTab,
    is_authenticated: bool,
    login_username: String,
    login_password: String,
    login_error: Option<String>,
}

impl NovaGuiApp {
    pub fn new() -> Self {
        // Initialize classroom state
        classroom::init_classroom();

        // Initialize chatbot history
        {
            let mut hist = ASSISTANT_HISTORY.lock().unwrap();
            if hist.is_none() {
                *hist = Some(vec![
                    ("assistant".to_string(), "Hello, I am Nova Assistant, your systems programming guide! Type 'help' to see topics.".to_string())
                ]);
            }
        }

        // Pre-populate some processes for visualization demo
        let _ = kernel::task::create_process("kswapd", kernel::task::ProcessPriority::High, vec![kernel::task::Capability::SysAdmin], 0);
        let _ = kernel::task::create_process("sshd", kernel::task::ProcessPriority::Normal, vec![kernel::task::Capability::NetworkRaw], 0);

        // Force logout on boot so the system starts at the login screen
        userspace::logout_user();

        NovaGuiApp {
            active_tab: Tab::KernelVisualizer,
            assistant_agent: assistant::NovaAssistant::new(),
            announcement_input: String::new(),
            chat_input: String::new(),
            selected_pid: None,
            teacher_window_open: false,
            teacher_active_tab: TeacherTab::ClassroomControl,
            is_authenticated: false,
            login_username: String::new(),
            login_password: String::new(),
            login_error: None,
        }
    }
}

impl eframe::App for NovaGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint every 100ms to keep dashboard refreshed (process ticks, logs, vga)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Force dark mode with a pure black background for all panels
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::BLACK;
        ctx.set_visuals(visuals);

        // Check if userspace logged out (i.e. CURRENT_SESSION_USER set to None via terminal logout command)
        let current_user = userspace::get_current_user();
        if current_user.is_none() {
            self.is_authenticated = false;
        }

        // Render login portal screen if not authenticated
        if !self.is_authenticated {
            self.draw_login_portal(ctx);
            return;
        }

        // Redirect active_tab if it is a teacher-only tab that has been moved to the separate window
        if self.active_tab == Tab::TeacherDashboard || self.active_tab == Tab::SecurityAuditor {
            self.active_tab = Tab::KernelVisualizer;
        }

        // Intercept keyboard events for VGA terminal if another text box is not focused
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                for event in &i.events {
                    match event {
                        egui::Event::Text(text) => {
                            if !classroom::is_terminal_locked() {
                                let mut buf = SHELL_INPUT_BUFFER.lock().unwrap();
                                for c in text.chars() {
                                    buf.push(c);
                                    drivers::vga_print!("{}", c);
                                }
                            }
                        }
                        egui::Event::Key { key, pressed: true, .. } => {
                            if !classroom::is_terminal_locked() {
                                match key {
                                    egui::Key::Backspace => {
                                        let mut buf = SHELL_INPUT_BUFFER.lock().unwrap();
                                        if !buf.is_empty() {
                                            buf.pop();
                                            // Erase from VGA text mode manually
                                            let mut cursor_lock = drivers::vga::VGA_CURSOR.lock();
                                            let (row, col) = *cursor_lock;
                                            if col > 0 {
                                                *cursor_lock = (row, col - 1);
                                                let offset = (row * drivers::vga::VGA_WIDTH + (col - 1)) * 2;
                                                let mut vga_buf = drivers::vga::VGA_BUFFER.lock();
                                                vga_buf[offset] = b' ';
                                            }
                                        }
                                    }
                                    egui::Key::Enter => {
                                        let mut buf = SHELL_INPUT_BUFFER.lock().unwrap();
                                        let cmd = buf.clone();
                                        drivers::vga_println!(); // new line
                                        buf.clear();
                                        drop(buf); // release lock before execution
                                        
                                        // Log syscall trigger trace
                                        let _ = kernel::syscall::sys_call(1, 0, 0, 0); // trigger dummy call to log action
                                        
                                        let output = shell::execute_command(&cmd);
                                        drivers::vga_print!("{}", output);
                                        
                                        // Print prompt line
                                        let active_user = userspace::get_current_user().unwrap().username;
                                        drivers::vga_print!("[{}@novaschool-os]$ ", active_user);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        let current_user = userspace::get_current_user();
        let is_admin = if let Some(ref u) = current_user {
            u.groups.contains(&"wheel".to_string()) || u.groups.contains(&"faculty".to_string()) || u.uid == 0
        } else {
            false
        };

        // Auto-close teacher viewport if active session is not administrative
        if !is_admin {
            self.teacher_window_open = false;
        }

        egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(
                        egui::RichText::new("🚀 NovaSchool Operating System Kernel")
                            .size(22.0)
                            .strong()
                            .color(egui::Color32::from_rgb(0, 220, 255))
                    );
                    let user_label = if let Some(ref u) = current_user {
                        format!("Interactive Kernel & Userspace | Logged in: {} (UID: {})", u.username, u.uid)
                    } else {
                        "Interactive Kernel & Userspace".to_string()
                    };
                    ui.label(egui::RichText::new(user_label).italics().color(egui::Color32::GRAY));
                });
                
                if is_admin {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if self.teacher_window_open {
                            "👨‍🏫 Teacher Panel: OPEN"
                        } else {
                            "👨‍🏫 Open Teacher Control Panel"
                        };
                        
                        let button = egui::Button::new(
                            egui::RichText::new(btn_text)
                                .strong()
                                .color(if self.teacher_window_open { egui::Color32::from_rgb(100, 255, 100) } else { egui::Color32::WHITE })
                        );
                        
                        if ui.add(button).clicked() {
                            self.teacher_window_open = !self.teacher_window_open;
                        }
                    });
                } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("🎓 Student Mode").color(egui::Color32::from_rgb(150, 150, 150)).strong());
                    });
                }
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left column: Simulated VGA console
                ui.vertical(|ui| {
                    ui.set_width(640.0);
                    let lock_status = if classroom::is_terminal_locked() {
                        "⚠️ TERMINAL LOCKED BY INSTRUCTOR"
                    } else {
                        "💻 Student VGA Interactive Console"
                    };
                    let title_color = if classroom::is_terminal_locked() {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::from_rgb(100, 255, 100)
                    };
                    
                    ui.label(egui::RichText::new(lock_status).color(title_color).strong().size(16.0));
                    ui.add_space(4.0);

                    // Black background console frame
                    egui::Frame::canvas(ui.style())
                        .fill(egui::Color32::BLACK)
                        .stroke(egui::Stroke::new(1.5_f32, title_color))
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.set_min_height(450.0);
                            ui.set_min_width(620.0);
                            
                            // Make the canvas region clickable to clear focus from other text entries
                            let rect = ui.max_rect();
                            let _response = ui.interact(rect, ui.id().with("vga_canvas_click"), egui::Sense::click());

                            // Render VGA text grid
                            let vga_data = drivers::vga::VGA_BUFFER.lock();
                            let cursor_pos = *drivers::vga::VGA_CURSOR.lock();
                            let time = ui.input(|i| i.time);
                            let show_cursor_blink = !ui.ctx().wants_keyboard_input() && ((time * 2.0) as i64 % 2 == 0);
                            
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.0;
                                ui.add_space(5.0);
                                for row in 0..drivers::vga::VGA_HEIGHT {
                                    draw_vga_row(ui, row, &*vga_data, cursor_pos, show_cursor_blink);
                                }
                                ui.add_space(5.0);
                            });
                        });
                    
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Click inside the app window and type directly to use the terminal. Use Ctrl+C if locked.").color(egui::Color32::GRAY).small());
                });

                // Spacing separator
                ui.add_space(16.0);

                // Right column: Dashboard Educational Tabs
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    // Tab selection bar (Student dashboard only)
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::KernelVisualizer, "📊 Visualizer");
                        ui.selectable_value(&mut self.active_tab, Tab::SyscallExplorer, "🔍 Syscalls");
                        ui.selectable_value(&mut self.active_tab, Tab::AiAssistant, "🤖 AI Assistant");
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Render active tab contents
                    match self.active_tab {
                        Tab::KernelVisualizer => {
                            self.draw_kernel_visualizer(ui);
                        }
                        Tab::SyscallExplorer => {
                            self.draw_syscall_explorer(ui);
                        }
                        Tab::TeacherDashboard => {
                            self.draw_teacher_dashboard(ui);
                        }
                        Tab::AiAssistant => {
                            self.draw_ai_assistant(ui);
                        }
                        Tab::SecurityAuditor => {
                            self.draw_security_auditor(ui);
                        }
                    }
                });
            });
        });

        // Manage separate Teacher & Admin Window (Viewport)
        if self.teacher_window_open {
            let mut open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("teacher_dashboard"),
                egui::ViewportBuilder::default()
                    .with_title("NovaSchool OS - Teacher & Admin Panel")
                    .with_inner_size([750.0, 550.0])
                    .with_close_button(true),
                |ctx, class| {
                    if class == egui::ViewportClass::Immediate {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            self.draw_teacher_admin_viewport(ui);
                        });
                        if ctx.input(|i| i.viewport().close_requested()) {
                            open = false;
                        }
                    }
                }
            );
            if !open {
                self.teacher_window_open = false;
            }
        }
    }
}

impl NovaGuiApp {
    fn draw_kernel_visualizer(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Process PCB Table
            ui.label(egui::RichText::new("Process Control Block (PCB) Table").strong().color(egui::Color32::from_rgb(0, 220, 255)));
            ui.label(egui::RichText::new("Click a process row to inspect details and manage execution:").small().color(egui::Color32::GRAY));
            ui.add_space(4.0);

            egui::Grid::new("pcb_table_gui").striped(true).spacing([15.0, 6.0]).show(ui, |ui| {
                ui.label(egui::RichText::new("PID").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                ui.label(egui::RichText::new("NAME").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                ui.label(egui::RichText::new("PRIO").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                ui.label(egui::RichText::new("STATE").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                ui.label(egui::RichText::new("TICKS").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                ui.end_row();
                
                let processes = kernel::task::get_process_list();
                for p in processes.iter().filter(|p| p.state != kernel::task::ProcessState::Killed) {
                    let is_selected = self.selected_pid == Some(p.pid);
                    let color = match p.state {
                        kernel::task::ProcessState::Running => egui::Color32::from_rgb(100, 255, 100),
                        kernel::task::ProcessState::Ready => egui::Color32::WHITE,
                        kernel::task::ProcessState::Blocked => egui::Color32::from_rgb(255, 220, 100),
                        _ => egui::Color32::GRAY,
                    };
                    
                    let p_pid = p.pid;
                    if ui.selectable_label(is_selected, egui::RichText::new(p.pid.to_string()).color(color)).clicked() {
                        self.selected_pid = Some(p_pid);
                    }
                    if ui.selectable_label(is_selected, egui::RichText::new(&p.name).color(color)).clicked() {
                        self.selected_pid = Some(p_pid);
                    }
                    ui.label(egui::RichText::new(format!("{:?}", p.priority)).color(color));
                    ui.label(egui::RichText::new(format!("{:?}", p.state)).color(color));
                    ui.label(egui::RichText::new(p.cpu_ticks.to_string()).color(color));
                    ui.end_row();
                }
            });

            // Process Inspector
            if let Some(pid) = self.selected_pid {
                let processes = kernel::task::get_process_list();
                if let Some(p) = processes.iter().find(|p| p.pid == pid) {
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("🔍 Process Inspector: PID {} ({})", p.pid, p.name)).strong().size(14.0).color(egui::Color32::from_rgb(255, 220, 100)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("❌ Close Inspector").clicked() {
                                    self.selected_pid = None;
                                }
                                
                                if p.state != kernel::task::ProcessState::Killed {
                                    if ui.button("💀 Kill Process").clicked() {
                                        let _ = kernel::task::kill_process(p.pid);
                                        self.selected_pid = None;
                                    }
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Execution Info").strong());
                                ui.label(format!("State: {:?}", p.state));
                                ui.label(format!("Priority: {:?}", p.priority));
                                ui.label(format!("Owner UID: {}", p.owner_uid));
                                ui.label(format!("CPU Ticks: {}", p.cpu_ticks));

                                // Calculate memory frame metrics
                                let (frames, page_counts) = kernel::mem::get_memory_snapshot();
                                let physical_frames_count = frames.iter().filter(|f| {
                                    match f {
                                        kernel::mem::FrameOwner::Process(owner_pid) => *owner_pid == p.pid,
                                        kernel::mem::FrameOwner::Cow(owner_pid) => *owner_pid == p.pid,
                                        _ => false
                                    }
                                }).count();
                                let virtual_pages = page_counts.get(&p.pid).copied().unwrap_or(0);
                                
                                ui.label(format!("Physical Memory: {} KB ({} frames)", physical_frames_count * 4, physical_frames_count));
                                ui.label(format!("Virtual Memory: {} KB ({} pages mapped)", virtual_pages * 4, virtual_pages));
                            });

                            ui.add_space(30.0);

                            // CPU register context (shows Rip, Rsp, etc.)
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("CPU Context (Registers)").strong());
                                ui.label(format!("  RIP: 0x{:08X}", p.context.rip));
                                ui.label(format!("  RSP: 0x{:08X}", p.context.rsp));
                                ui.label(format!("  RAX: 0x{:X}", p.context.rax));
                                ui.label(format!("  RBX: 0x{:X}", p.context.rbx));
                                ui.label(format!("  RDI: 0x{:X}", p.context.rdi));
                                ui.label(format!("  RSI: 0x{:X}", p.context.rsi));
                            });

                            ui.add_space(30.0);

                            // Process Capabilities
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Process Capabilities").strong());
                                if p.capabilities.is_empty() {
                                    ui.label("  None (Unprivileged)");
                                } else {
                                    for cap in &p.capabilities {
                                        ui.label(format!("  - {:?}", cap));
                                    }
                                }
                            });

                            ui.add_space(30.0);

                            // Open File Descriptors for this process
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Open Files (FDs)").strong());
                                let open_files = filesystem::get_vfs_open_files();
                                let process_fds: Vec<_> = open_files.into_iter().filter(|(fd, _)| p.file_descriptors.contains(fd)).collect();
                                if process_fds.is_empty() {
                                    ui.label("  None");
                                } else {
                                    for (fd, desc) in process_fds {
                                        ui.label(format!("  FD {}: {}", fd, desc.path));
                                    }
                                }
                            });
                        });
                    });
                } else {
                    self.selected_pid = None; // Reset if process no longer exists
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // System Interrupt Counters
            ui.label(egui::RichText::new("System Interrupt Counters").strong().color(egui::Color32::from_rgb(0, 220, 255)));
            ui.add_space(4.0);

            let (timer, kbd, pf) = kernel::interrupts::get_interrupt_stats();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Timer Interrupts (IRQ 0):");
                    ui.strong(egui::RichText::new(timer.to_string()).color(egui::Color32::from_rgb(100, 180, 255)));
                });
                ui.horizontal(|ui| {
                    ui.label("Keyboard Interrupts (IRQ 1):");
                    ui.strong(egui::RichText::new(kbd.to_string()).color(egui::Color32::from_rgb(100, 255, 100)));
                });
                ui.horizontal(|ui| {
                    ui.label("Page Fault Exceptions (Vector 14):");
                    ui.strong(egui::RichText::new(pf.to_string()).color(egui::Color32::from_rgb(255, 100, 100)));
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // VFS Mounts & Open File Descriptors
            ui.label(egui::RichText::new("Virtual File System (VFS)").strong().color(egui::Color32::from_rgb(0, 220, 255)));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                // Left side: Mount Points Table
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Active Mount Points").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                    ui.add_space(4.0);
                    egui::Grid::new("vfs_mounts").striped(true).spacing([20.0, 4.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("Mount Path").strong());
                        ui.label(egui::RichText::new("Filesystem").strong());
                        ui.end_row();

                        let mounts = filesystem::get_vfs_mount_points();
                        for m in mounts {
                            ui.label(&m.path);
                            ui.label(format!("{:?}", m.mtype));
                            ui.end_row();
                        }
                    });
                });

                ui.add_space(40.0);

                // Right side: Open File Descriptors
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Open File Descriptors").strong().color(egui::Color32::from_rgb(255, 220, 100)));
                    ui.add_space(4.0);
                    egui::Grid::new("vfs_fds").striped(true).spacing([20.0, 4.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("FD").strong());
                        ui.label(egui::RichText::new("Target Path").strong());
                        ui.label(egui::RichText::new("Access").strong());
                        ui.end_row();

                        let fds = filesystem::get_vfs_open_files();
                        for (fd, desc) in fds {
                            ui.label(fd.to_string());
                            ui.label(&desc.path);
                            let access = match (desc.readable, desc.writable) {
                                (true, true) => "Read / Write",
                                (true, false) => "Read-Only",
                                (false, true) => "Write-Only",
                                (false, false) => "Closed",
                            };
                            ui.label(access);
                            ui.end_row();
                        }
                    });
                });
            });
        });
    }

    fn draw_syscall_explorer(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("POSIX Syscall Trace Explorer").strong().color(egui::Color32::from_rgb(0, 220, 255)));
        ui.label(egui::RichText::new("Recent syscall interface invocations logs:").small().color(egui::Color32::GRAY));
        ui.add_space(6.0);

        let traces = kernel::syscall::get_syscall_traces();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for trace in traces.iter().rev() {
                let result_text = match &trace.result {
                    Ok(val) => format!("SUCCESS (ret: {})", val),
                    Err(e) => format!("ERROR: {}", e),
                };
                let color = if trace.result.is_ok() {
                    egui::Color32::from_rgb(100, 255, 100)
                } else {
                    egui::Color32::from_rgb(255, 100, 100)
                };
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(format!(
                        "PID {}: {} (arg1: {}, arg2: {}, arg3: {})", 
                        trace.pid, trace.syscall_name, trace.args[0], trace.args[1], trace.args[2]
                    )).strong());
                    ui.label(egui::RichText::new(format!("  -> Result: {}", result_text)).color(color));
                });
                ui.add_space(4.0);
            }
        });
    }

    fn draw_teacher_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Classroom Management Panel").strong().color(egui::Color32::from_rgb(0, 220, 255)));
        ui.add_space(6.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Student Terminal Interactivity Lock").strong());
            ui.add_space(4.0);
            let locked = classroom::is_terminal_locked();
            ui.horizontal(|ui| {
                if locked {
                    ui.label(egui::RichText::new("Status: LOCKED").color(egui::Color32::from_rgb(255, 100, 100)).strong());
                    if ui.button("🔓 Unlock Student Input").clicked() {
                        classroom::set_terminal_lock(false);
                        classroom::broadcast_announcement("Student terminals unlocked.");
                    }
                } else {
                    ui.label(egui::RichText::new("Status: ACTIVE").color(egui::Color32::from_rgb(100, 255, 100)).strong());
                    if ui.button("🔒 Lock Student Input").clicked() {
                        classroom::set_terminal_lock(true);
                        classroom::broadcast_announcement("TERMINAL LOCKED BY INSTRUCTOR. Pay attention to board.");
                    }
                }
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Kernel & OS Control Actions").strong());
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("📂 Distribute Lab 1 Handout").on_hover_text("Writes lab1_handout.txt into student workspaces").clicked() {
                    let _ = classroom::distribute_lab_assignment();
                }
                if ui.button("🧹 Reset Student Workspace").on_hover_text("Wipes clean student workspace directory structures").clicked() {
                    userspace::auto_reset_student_environment(1001);
                    classroom::broadcast_announcement("Pristine workspace environment restored.");
                }
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Broadcast Classroom Announcement").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.announcement_input);
                if ui.button("📢 Broadcast").clicked() {
                    if !self.announcement_input.is_empty() {
                        classroom::broadcast_announcement(&self.announcement_input);
                        self.announcement_input.clear();
                    }
                }
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(egui::RichText::new("Recent Classroom Alerts Log").strong());
        ui.add_space(4.0);

        let announcements = classroom::get_announcements();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for ann in announcements.iter().rev() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📢").color(egui::Color32::from_rgb(255, 100, 100)));
                    ui.label(ann);
                });
            }
        });
    }

    fn draw_ai_assistant(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Nova School AI Assistant Chatbot").strong().color(egui::Color32::from_rgb(0, 220, 255)));
        ui.label(egui::RichText::new("Ask concepts e.g. scheduler, memory, novafs, syscall, ls, cat...").small().color(egui::Color32::GRAY));
        ui.add_space(6.0);

        let mut hist_lock = ASSISTANT_HISTORY.lock().unwrap();
        let hist = hist_lock.as_mut().unwrap();

        // Chat Log Scroll Area
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                for (sender, text) in hist.iter() {
                    let is_user = sender == "user";
                    let (label, color) = if is_user {
                        ("🎓 Student", egui::Color32::from_rgb(100, 255, 100))
                    } else {
                        ("🤖 Assistant", egui::Color32::from_rgb(100, 180, 255))
                    };
                    
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(label).color(color).strong());
                        ui.label(text);
                    });
                    ui.add_space(4.0);
                }
            });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Prompt Input
        ui.horizontal(|ui| {
            let res = ui.text_edit_singleline(&mut self.chat_input);
            
            // Check for Enter key inside text box
            let enter_pressed = res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            
            if ui.button("Send").clicked() || enter_pressed {
                if !self.chat_input.is_empty() {
                    let query = self.chat_input.clone();
                    self.chat_input.clear();
                    
                    let reply = self.assistant_agent.ask(&query);
                    if hist.len() > 10 {
                        hist.remove(0);
                    }
                    hist.push(("user".to_string(), query));
                    hist.push(("assistant".to_string(), reply));
                }
            }
        });
    }

    fn draw_security_auditor(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("MAC & Intrusion Audit Log").strong().color(egui::Color32::from_rgb(0, 220, 255)));
        ui.label(egui::RichText::new("Mandatory Access Control (MAC) access events log:").small().color(egui::Color32::GRAY));
        ui.add_space(6.0);

        let logs = kernel::security::get_security_audit_logs();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for log in logs.iter().rev() {
                let is_denied = log.status == "DENIED";
                let color = if is_denied {
                    egui::Color32::from_rgb(255, 100, 100)
                } else {
                    egui::Color32::from_rgb(100, 255, 100)
                };
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let icon = if is_denied { "❌ [VIOLATION]" } else { "🛡️ [AUDIT]" };
                        ui.label(egui::RichText::new(icon).color(color).strong());
                        ui.label(format!("UID {} | Action: {} | Status: {}", log.user_uid, log.action, log.status));
                    });
                    ui.label(egui::RichText::new(format!("  Details: {}", log.details)).color(egui::Color32::GRAY));
                });
                ui.add_space(4.0);
            }
        });
    }

    fn draw_teacher_admin_viewport(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.heading(
                egui::RichText::new("👨‍🏫 NovaSchool OS Teacher & Admin Panel")
                    .size(20.0)
                    .strong()
                    .color(egui::Color32::from_rgb(255, 220, 100))
            );
            ui.label("Secure Classroom Management & Intrusion Auditing Console");
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Tab selection for Teacher Dashboard
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.teacher_active_tab, TeacherTab::ClassroomControl, "👨‍🏫 Classroom Control");
                ui.selectable_value(&mut self.teacher_active_tab, TeacherTab::SecurityAuditor, "🛡️ Security Auditor");
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Render active sub-tab contents
            match self.teacher_active_tab {
                TeacherTab::ClassroomControl => {
                    self.draw_teacher_dashboard(ui);
                }
                TeacherTab::SecurityAuditor => {
                    self.draw_security_auditor(ui);
                }
            }
        });
    }

    fn draw_login_portal(&mut self, ctx: &egui::Context) {
        // Render a centered glassmorphism box on a pure black background
        let panel_frame = egui::Frame::default()
            .fill(egui::Color32::BLACK)
            .inner_margin(8.0);

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.20);
                
                // OS Logo or Icon
                ui.label(
                    egui::RichText::new("🚀")
                        .size(60.0)
                );
                ui.add_space(8.0);
                
                // OS Name
                ui.heading(
                    egui::RichText::new("NovaSchool OS")
                        .size(32.0)
                        .strong()
                        .color(egui::Color32::from_rgb(0, 220, 255))
                );
                ui.label(
                    egui::RichText::new("Secure Educational Operating System & Kernel")
                        .color(egui::Color32::GRAY)
                        .italics()
                );
                
                ui.add_space(24.0);

                // Login form container - sleek dark premium design with subtle border
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(18, 18, 18))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(32, 32, 32)))
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(egui::Margin::symmetric(24.0, 20.0))
                    .show(ui, |ui| {
                        ui.set_width(320.0);
                        ui.vertical(|ui| {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Login Portal").strong().size(18.0).color(egui::Color32::WHITE));
                            ui.add_space(16.0);
                            
                            // Style configuration for high-contrast, premium rounded inputs
                            ui.style_mut().visuals.extreme_bg_color = egui::Color32::from_rgb(26, 26, 26);
                            ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(55, 55, 55));
                            ui.style_mut().visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
                            ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 220, 255));
                            ui.style_mut().visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
                            ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 220, 255));
                            ui.style_mut().visuals.widgets.active.rounding = egui::Rounding::same(6.0);

                            // Username Input
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("👤 Username").color(egui::Color32::from_rgb(180, 180, 180)).strong().size(13.0));
                            });
                            ui.add_space(6.0);
                            let text_username = ui.add(
                                egui::TextEdit::singleline(&mut self.login_username)
                                    .desired_width(f32::INFINITY)
                                    .margin(egui::Margin::symmetric(10.0, 8.0))
                                    .hint_text("e.g. student1, teacher, root")
                            );
                            
                            ui.add_space(14.0);
                            
                            // Password Input
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🔑 Password").color(egui::Color32::from_rgb(180, 180, 180)).strong().size(13.0));
                            });
                            ui.add_space(6.0);
                            let text_password = ui.add(
                                egui::TextEdit::singleline(&mut self.login_password)
                                    .desired_width(f32::INFINITY)
                                    .margin(egui::Margin::symmetric(10.0, 8.0))
                                    .password(true)
                                    .hint_text("••••••••")
                            );
                            
                            ui.add_space(20.0);

                            // Log In button - stylish cyan accent with modern rounded corners
                            let button = egui::Button::new(
                                egui::RichText::new("Authenticate & Boot")
                                    .strong()
                                    .size(14.0)
                                    .color(egui::Color32::BLACK)
                            ).fill(egui::Color32::from_rgb(0, 220, 255));
                            
                            // Detect Enter key in fields or button click
                            let enter_pressed = (text_username.lost_focus() || text_password.lost_focus())
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));

                            if ui.add_sized([ui.available_width(), 36.0], button).clicked() || enter_pressed {
                                if self.login_username.is_empty() || self.login_password.is_empty() {
                                    self.login_error = Some("Please fill in all fields".to_string());
                                } else {
                                    match userspace::login_user(&self.login_username, &self.login_password) {
                                        Ok(_) => {
                                            self.is_authenticated = true;
                                            self.login_error = None;
                                            // Reset the VGA console so it starts with a clean shell prompt
                                            drivers::vga::init_vga();
                                            kernel::boot::print_boot_banner();
                                            kernel::boot::print_uefi_memory_map();
                                            drivers::vga_println!("Type 'help' to see shell commands. Nav tab views using tabs on the right.");
                                            let active_user = userspace::get_current_user().unwrap().username;
                                            drivers::vga_print!("[{}@novaschool-os]$ ", active_user);
                                        }
                                        Err(e) => {
                                            self.login_error = Some(e);
                                        }
                                    }
                                }
                            }
                            
                            // Display error if any
                            if let Some(ref err) = self.login_error {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(format!("❌ {}", err)).color(egui::Color32::from_rgb(255, 100, 100)).small());
                            }
                            
                            ui.add_space(4.0);
                        });
                    });
                
                ui.add_space(30.0);
                
                // Faculty/Student default credentials help labels to guide users
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("🔑 Quick Access Accounts:").color(egui::Color32::from_rgb(120, 120, 120)).small());
                    ui.label(egui::RichText::new("student1/student123").color(egui::Color32::from_rgb(160, 160, 160)).small());
                    ui.label(egui::RichText::new(" | ").color(egui::Color32::from_rgb(80, 80, 80)).small());
                    ui.label(egui::RichText::new("teacher/teacher123").color(egui::Color32::from_rgb(160, 160, 160)).small());
                    ui.label(egui::RichText::new(" | ").color(egui::Color32::from_rgb(80, 80, 80)).small());
                    ui.label(egui::RichText::new("root/admin123").color(egui::Color32::from_rgb(160, 160, 160)).small());
                });
            });
        });
    }
}

fn draw_vga_row(ui: &mut egui::Ui, row: usize, vga_data: &[u8], cursor_pos: (usize, usize), show_cursor_blink: bool) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 0.0;
        let mut col = 0;
        while col < 80 {
            let offset = (row * 80 + col) * 2;
            let first_attr = vga_data[offset + 1];
            
            // Check if this cell is the blinking hardware cursor
            let is_cursor = row == cursor_pos.0 && col == cursor_pos.1 && show_cursor_blink;
            
            let mut text = String::new();
            if is_cursor {
                let ch = vga_data[offset];
                let character = if ch == 0 || ch == b'\n' {
                    ' '
                } else {
                    ch as char
                };
                text.push(character);
                col += 1;
                
                // Override to draw blinking green block cursor
                let fg_color = egui::Color32::BLACK;
                let bg_color = egui::Color32::from_rgb(100, 255, 100);
                
                let font_id = egui::FontId::monospace(13.0);
                let mut job = egui::text::LayoutJob::default();
                let text_format = egui::TextFormat {
                    font_id,
                    color: fg_color,
                    background: bg_color,
                    ..Default::default()
                };
                job.append(&text, 0.0, text_format);
                ui.label(job);
            } else {
                // Gather contiguous characters with same attributes, stopping at the cursor
                while col < 80 {
                    if row == cursor_pos.0 && col == cursor_pos.1 && show_cursor_blink {
                        break;
                    }
                    
                    let curr_offset = (row * 80 + col) * 2;
                    let ch = vga_data[curr_offset];
                    let attr = vga_data[curr_offset + 1];
                    if attr != first_attr {
                        break;
                    }
                    
                    let character = if ch == 0 || ch == b'\n' {
                        ' '
                    } else {
                        ch as char
                    };
                    text.push(character);
                    col += 1;
                }
                
                let fg_color = vga_color_to_egui(first_attr & 0x0F);
                let bg_color = vga_color_to_egui((first_attr & 0xF0) >> 4);
                
                let font_id = egui::FontId::monospace(13.0);
                let mut job = egui::text::LayoutJob::default();
                let text_format = egui::TextFormat {
                    font_id,
                    color: fg_color,
                    background: bg_color,
                    ..Default::default()
                };
                job.append(&text, 0.0, text_format);
                ui.label(job);
            }
        }
    });
}

fn vga_color_to_egui(color_val: u8) -> egui::Color32 {
    match color_val {
        0 => egui::Color32::from_rgb(0, 0, 0),         // Black
        1 => egui::Color32::from_rgb(0, 0, 170),       // Blue
        2 => egui::Color32::from_rgb(0, 170, 0),       // Green
        3 => egui::Color32::from_rgb(0, 170, 170),     // Cyan
        4 => egui::Color32::from_rgb(170, 0, 0),       // Red
        5 => egui::Color32::from_rgb(170, 0, 170),     // Magenta
        6 => egui::Color32::from_rgb(170, 85, 0),      // Brown
        7 => egui::Color32::from_rgb(170, 170, 170),   // Light Gray
        8 => egui::Color32::from_rgb(85, 85, 85),      // Dark Gray
        9 => egui::Color32::from_rgb(85, 85, 255),     // Light Blue
        10 => egui::Color32::from_rgb(85, 255, 85),    // Light Green
        11 => egui::Color32::from_rgb(85, 255, 255),   // Light Cyan
        12 => egui::Color32::from_rgb(255, 85, 85),    // Light Red
        13 => egui::Color32::from_rgb(255, 85, 255),   // Pink
        14 => egui::Color32::from_rgb(255, 255, 85),   // Yellow
        _ => egui::Color32::from_rgb(255, 255, 255),  // White
    }
}
