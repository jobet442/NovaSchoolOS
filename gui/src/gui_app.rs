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

pub struct NovaGuiApp {
    active_tab: Tab,
    assistant_agent: assistant::NovaAssistant,
    announcement_input: String,
    chat_input: String,
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

        // Boot banner prints
        kernel::boot::print_boot_banner();
        kernel::boot::print_uefi_memory_map();
        drivers::vga_println!("Type 'help' to see shell commands. Nav tab views using tabs on the right.");

        // Print initial prompt line
        let active_user = userspace::get_current_user().unwrap().username;
        drivers::vga_print!("[{}@novaschool-os]$ ", active_user);

        NovaGuiApp {
            active_tab: Tab::KernelVisualizer,
            assistant_agent: assistant::NovaAssistant::new(),
            announcement_input: String::new(),
            chat_input: String::new(),
        }
    }
}

impl eframe::App for NovaGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint every 100ms to keep dashboard refreshed (process ticks, logs, vga)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

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

        egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(
                    egui::RichText::new("🚀 NovaSchool OS Operating System Simulator")
                        .size(24.0)
                        .strong()
                        .color(egui::Color32::from_rgb(0, 220, 255))
                );
                ui.label(egui::RichText::new("Interactive Educational Operating System Sandbox").italics().color(egui::Color32::GRAY));
                ui.add_space(8.0);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                // Left column: Simulated VGA console
                let ui_vga = &mut columns[0];
                ui_vga.vertical(|ui| {
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
                            
                            // Render VGA text grid
                            let vga_data = drivers::vga::VGA_BUFFER.lock();
                            
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.0;
                                ui.add_space(5.0);
                                for row in 0..drivers::vga::VGA_HEIGHT {
                                    draw_vga_row(ui, row, &*vga_data);
                                }
                                ui.add_space(5.0);
                            });
                        });
                    
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Click inside the app window and type directly to use the terminal. Use Ctrl+C if locked.").color(egui::Color32::GRAY).small());
                });

                // Right column: Dashboard Educational Tabs
                let ui_tabs = &mut columns[1];
                ui_tabs.vertical(|ui| {
                    // Tab selection bar
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::KernelVisualizer, "📊 Visualizer");
                        ui.selectable_value(&mut self.active_tab, Tab::SyscallExplorer, "🔍 Syscalls");
                        ui.selectable_value(&mut self.active_tab, Tab::TeacherDashboard, "👨‍🏫 Teacher");
                        ui.selectable_value(&mut self.active_tab, Tab::AiAssistant, "🤖 AI Assistant");
                        ui.selectable_value(&mut self.active_tab, Tab::SecurityAuditor, "🛡️ Auditor");
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
    }
}

impl NovaGuiApp {
    fn draw_kernel_visualizer(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(egui::RichText::new("Physical Memory Allocation Grid (1MB RAM)").strong().color(egui::Color32::from_rgb(0, 220, 255)));
            ui.label(egui::RichText::new("Hover squares to inspect memory frame status:").small().color(egui::Color32::GRAY));
            ui.add_space(4.0);

            // Memory Grid
            egui::Grid::new("mem_grid").spacing([3.0, 3.0]).show(ui, |ui| {
                let (frames, _) = kernel::mem::get_memory_snapshot();
                for row in 0..16 {
                    for col in 0..16 {
                        let idx = row * 16 + col;
                        let frame = frames.get(idx).unwrap_or(&kernel::mem::FrameOwner::Free);
                        let color = match frame {
                            kernel::mem::FrameOwner::Free => egui::Color32::from_rgb(45, 45, 45),
                            kernel::mem::FrameOwner::Kernel => egui::Color32::from_rgb(220, 50, 50),
                            kernel::mem::FrameOwner::Process(pid) => {
                                match pid % 5 {
                                    0 => egui::Color32::from_rgb(50, 180, 50),
                                    1 => egui::Color32::from_rgb(100, 220, 100),
                                    2 => egui::Color32::from_rgb(50, 150, 220),
                                    3 => egui::Color32::from_rgb(180, 50, 180),
                                    _ => egui::Color32::from_rgb(80, 200, 200),
                                }
                            }
                            kernel::mem::FrameOwner::Shared => egui::Color32::from_rgb(0, 180, 220),
                            kernel::mem::FrameOwner::Cow(_) => egui::Color32::from_rgb(220, 180, 0),
                        };
                        
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(14.0, 14.0), 
                            egui::Sense::hover()
                        );
                        ui.painter().rect_filled(rect, 2.0, color);
                        
                        response.on_hover_text(format!("Frame {}: {:?}", idx, frame));
                    }
                    ui.end_row();
                }
            });

            // Grid Legend
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.small("Legend: ");
                legend_item(ui, "Free", egui::Color32::from_rgb(45, 45, 45));
                legend_item(ui, "Kernel", egui::Color32::from_rgb(220, 50, 50));
                legend_item(ui, "Process", egui::Color32::from_rgb(50, 180, 50));
                legend_item(ui, "Shared", egui::Color32::from_rgb(0, 180, 220));
                legend_item(ui, "COW (Copy On Write)", egui::Color32::from_rgb(220, 180, 0));
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Process PCB Table
            ui.label(egui::RichText::new("Process Control Block (PCB) Table").strong().color(egui::Color32::from_rgb(0, 220, 255)));
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
                    let color = match p.state {
                        kernel::task::ProcessState::Running => egui::Color32::from_rgb(100, 255, 100),
                        kernel::task::ProcessState::Ready => egui::Color32::WHITE,
                        kernel::task::ProcessState::Blocked => egui::Color32::from_rgb(255, 220, 100),
                        _ => egui::Color32::GRAY,
                    };
                    ui.label(egui::RichText::new(p.pid.to_string()).color(color));
                    ui.label(egui::RichText::new(&p.name).color(color));
                    ui.label(egui::RichText::new(format!("{:?}", p.priority)).color(color));
                    ui.label(egui::RichText::new(format!("{:?}", p.state)).color(color));
                    ui.label(egui::RichText::new(p.cpu_ticks.to_string()).color(color));
                    ui.end_row();
                }
            });

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
            ui.label(egui::RichText::new("Simulation Sandbox Control Actions").strong());
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
}

fn legend_item(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 1.0, color);
        ui.small(text);
    });
}

fn draw_vga_row(ui: &mut egui::Ui, row: usize, vga_data: &[u8]) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 0.0;
        let mut col = 0;
        while col < 80 {
            let offset = (row * 80 + col) * 2;
            let first_attr = vga_data[offset + 1];
            
            // Gather contiguous characters with same attributes
            let mut text = String::new();
            while col < 80 {
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
            
            // Render text with specific background/foreground
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
