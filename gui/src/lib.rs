#[cfg(feature = "tui")]
pub mod widgets;
pub mod classroom;
pub mod assistant;

#[cfg(feature = "gui-window")]
pub mod gui_app;

#[cfg(feature = "tui")]
use std::io;
#[cfg(feature = "tui")]
use std::time::{Duration, Instant};
#[cfg(feature = "tui")]
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers};
#[cfg(feature = "tui")]
use crossterm::execute;
#[cfg(feature = "tui")]
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
#[cfg(feature = "tui")]
use ratatui::backend::CrosstermBackend;
#[cfg(feature = "tui")]
use ratatui::widgets::{Block, Borders, Paragraph, List, ListItem, Tabs};
#[cfg(feature = "tui")]
use ratatui::layout::{Layout, Constraint, Direction, Rect};
#[cfg(feature = "tui")]
use ratatui::style::{Style, Color, Modifier};
#[cfg(feature = "tui")]
use ratatui::text::{Span, Line};
#[cfg(feature = "tui")]
use ratatui::Terminal;

#[cfg(feature = "tui")]
static ACTIVE_TAB: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// Global shell input line buffer
pub(crate) static SHELL_INPUT_BUFFER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
// Global chatbot input line buffer
#[cfg(feature = "tui")]
pub(crate) static ASSISTANT_INPUT_BUFFER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
// Chatbot conversation log
pub(crate) static ASSISTANT_HISTORY: std::sync::Mutex<Option<Vec<(String, String)>>> = std::sync::Mutex::new(None);
// Teacher dashboard input line buffer
#[cfg(feature = "tui")]
pub(crate) static TEACHER_INPUT_BUFFER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

#[cfg(feature = "tui")]
pub fn start_tui_environment() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize state databases
    classroom::init_classroom();
    *ASSISTANT_HISTORY.lock().unwrap() = Some(vec![
        ("assistant".to_string(), "Hello, I am Nova Assistant, your systems programming guide! Type 'help' to see topics.".to_string())
    ]);

    // Pre-populate some processes for visualization demo
    let _ = kernel::task::create_process("kswapd", kernel::task::ProcessPriority::High, vec![kernel::task::Capability::SysAdmin], 0);
    let _ = kernel::task::create_process("sshd", kernel::task::ProcessPriority::Normal, vec![kernel::task::Capability::NetworkRaw], 0);

    // Boot banner prints
    kernel::boot::print_boot_banner();
    kernel::boot::print_uefi_memory_map();
    drivers::vga_println!("Type 'help' to see shell commands. Nav tab views using F1-F5.\n");

    let assistant_agent = assistant::NovaAssistant::new();
    let tick_rate = Duration::from_millis(150);
    let mut last_tick = Instant::now();

    loop {
        // Run network card background loops and schedule process increments
        networking::poll_network_card();
        
        // Accumulate CPU ticks on random active process to simulate preemptive timers
        let curr_p = kernel::task::get_current_pid();
        kernel::scheduler::increment_task_ticks(curr_p);

        // Periodically trigger a virtual timer interrupt for round-robin switching demo
        if last_tick.elapsed() >= Duration::from_secs(4) {
            kernel::interrupts::trigger_interrupt_vector(0x20, 0);
        }

        terminal.draw(|f| draw_dashboard(f, &assistant_agent))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Global hotkeys: F1 to F5 switch tabs
                match key.code {
                    KeyCode::F(1) => ACTIVE_TAB.store(0, std::sync::atomic::Ordering::SeqCst),
                    KeyCode::F(2) => ACTIVE_TAB.store(1, std::sync::atomic::Ordering::SeqCst),
                    KeyCode::F(3) => ACTIVE_TAB.store(2, std::sync::atomic::Ordering::SeqCst),
                    KeyCode::F(4) => ACTIVE_TAB.store(3, std::sync::atomic::Ordering::SeqCst),
                    KeyCode::F(5) => ACTIVE_TAB.store(4, std::sync::atomic::Ordering::SeqCst),
                    
                    // ESC to exit simulator
                    KeyCode::Esc => {
                        break;
                    }
                    
                    // Route input events based on active tab
                    _ => {
                        let active_tab = ACTIVE_TAB.load(std::sync::atomic::Ordering::SeqCst);
                        if active_tab == 0 {
                            // Shell Tab Input
                            if classroom::is_terminal_locked() {
                                // Locked notification prints
                                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // allows clearing block
                                    classroom::set_terminal_lock(false);
                                    drivers::vga_println!("\n[System Alert] Student terminal unlocked by override key combo.");
                                }
                                continue;
                            }
                            
                            let mut buf = SHELL_INPUT_BUFFER.lock().unwrap();
                            match key.code {
                                KeyCode::Char(c) => {
                                    buf.push(c);
                                    // Echo to VGA
                                    drivers::vga_print!("{}", c);
                                }
                                KeyCode::Backspace => {
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
                                KeyCode::Enter => {
                                    let cmd = buf.clone();
                                    drivers::vga_println!(); // new line
                                    buf.clear();
                                    
                                    // Execute in shell
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
                        } else if active_tab == 2 {
                            // Teacher Dashboard Input
                            let mut buf = TEACHER_INPUT_BUFFER.lock().unwrap();
                            match key.code {
                                KeyCode::Char(c) => {
                                    buf.push(c);
                                }
                                KeyCode::Backspace => {
                                    buf.pop();
                                }
                                KeyCode::Enter => {
                                    let cmd = buf.clone();
                                    buf.clear();
                                    drop(buf);

                                    // Run teacher terminal operations
                                    if cmd == "lock" {
                                        classroom::set_terminal_lock(true);
                                        classroom::broadcast_announcement("TERMINAL LOCKED BY INSTRUCTOR. Pay attention to board.");
                                    } else if cmd == "unlock" {
                                        classroom::set_terminal_lock(false);
                                        classroom::broadcast_announcement("Student terminals unlocked.");
                                    } else if cmd == "distribute" {
                                        let _ = classroom::distribute_lab_assignment();
                                    } else if cmd == "reset" {
                                        userspace::auto_reset_student_environment(1001);
                                        classroom::broadcast_announcement("Pristine workspace environment restored.");
                                    } else if cmd.starts_with("msg ") {
                                        let announcement = &cmd[4..];
                                        classroom::broadcast_announcement(announcement);
                                    }
                                }
                                _ => {}
                            }
                        } else if active_tab == 3 {
                            // Assistant Chatbot Input
                            let mut buf = ASSISTANT_INPUT_BUFFER.lock().unwrap();
                            match key.code {
                                KeyCode::Char(c) => {
                                    buf.push(c);
                                }
                                KeyCode::Backspace => {
                                    buf.pop();
                                }
                                KeyCode::Enter => {
                                    let query = buf.clone();
                                    buf.clear();
                                    drop(buf);
                                    
                                    let reply = assistant_agent.ask(&query);
                                    let mut hist = ASSISTANT_HISTORY.lock().unwrap();
                                    if let Some(ref mut vector) = *hist {
                                        if vector.len() > 10 {
                                            vector.remove(0);
                                        }
                                        vector.push(("user".to_string(), query));
                                        vector.push(("assistant".to_string(), reply));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(feature = "tui")]
fn draw_dashboard(f: &mut ratatui::Frame, _assistant: &assistant::NovaAssistant) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs header
            Constraint::Min(10),   // Content
        ])
        .split(f.size());

    // Render navigation tabs
    let active = ACTIVE_TAB.load(std::sync::atomic::Ordering::SeqCst);
    let tab_titles = vec![
        " [F1] Student Terminal ", 
        " [F2] Syscall Explorer ", 
        " [F3] Teacher Dashboard ", 
        " [F4] Nova AI Assistant ", 
        " [F5] Security Auditor "
    ];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" NovaSchool OS Dashboard "))
        .select(active)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, chunks[0]);

    // Split workspace: Left 60% is VGA output screen, Right 40% is selected educational view panel
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // VGA screen
            Constraint::Percentage(45), // Educational widget
        ])
        .split(chunks[1]);

    // 1. Draw VGA Framebuffer Console (Left Panel)
    draw_vga_console(f, content_chunks[0]);

    // 2. Draw active tab content (Right Panel)
    match active {
        0 => draw_kernel_visualizer_tab(f, content_chunks[1]),
        1 => draw_syscall_explorer_tab(f, content_chunks[1]),
        2 => draw_teacher_dashboard_tab(f, content_chunks[1]),
        3 => draw_assistant_tab(f, content_chunks[1]),
        _ => draw_security_auditor_tab(f, content_chunks[1]),
    }
}

// Renders the simulated VGA buffer memory character-by-character into Ratatui paragraph
#[cfg(feature = "tui")]
fn draw_vga_console(f: &mut ratatui::Frame, area: Rect) {
    let vga_data = drivers::vga::VGA_BUFFER.lock();
    let mut lines = Vec::new();

    for row in 0..drivers::vga::VGA_HEIGHT {
        let mut line_spans = Vec::new();
        for col in 0..drivers::vga::VGA_WIDTH {
            let offset = (row * drivers::vga::VGA_WIDTH + col) * 2;
            let ch = vga_data[offset];
            let attr = vga_data[offset + 1];

            // Decode VGA attributes to Ratatui Colors
            let fg = match attr & 0x0F {
                0 => Color::Black,
                1 => Color::Blue,
                2 => Color::Green,
                3 => Color::Cyan,
                4 => Color::Red,
                5 => Color::Magenta,
                6 => Color::Rgb(139, 69, 19),
                7 => Color::Gray,
                8 => Color::DarkGray,
                9 => Color::LightBlue,
                10 => Color::LightGreen,
                11 => Color::LightCyan,
                12 => Color::LightRed,
                13 => Color::LightMagenta,
                14 => Color::Yellow,
                _ => Color::White,
            };

            let character = if ch == 0 || ch == b'\n' {
                " ".to_string()
            } else {
                (ch as char).to_string()
            };
            line_spans.push(Span::styled(character, Style::default().fg(fg)));
        }
        lines.push(Line::from(line_spans));
    }
    let lock_status = if classroom::is_terminal_locked() {
        " [TERMINAL LOCKED - PRESS CTRL+C TO OVERRIDE] "
    } else {
        " [NovaSchool Interactive Terminal] "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(lock_status)
        .border_style(if classroom::is_terminal_locked() { Style::default().fg(Color::Red) } else { Style::default().fg(Color::LightGreen) });

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

// Tab 1: Kernel Visualizer (renders memory grid, scheduler queue, interrupts)
#[cfg(feature = "tui")]
fn draw_kernel_visualizer_tab(f: &mut ratatui::Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(18), // Memory grid
            Constraint::Min(6),     // Processes PCB table
            Constraint::Length(5),  // System Interrupt counts
        ])
        .split(area);

    let (frames, _) = kernel::mem::get_memory_snapshot();
    widgets::render_memory_grid(f, chunks[0], &frames);

    let processes = kernel::task::get_process_list();
    widgets::render_processes_table(f, chunks[1], &processes);

    widgets::render_interrupts(f, chunks[2]);
}

// Tab 2: Syscall Explorer
#[cfg(feature = "tui")]
fn draw_syscall_explorer_tab(f: &mut ratatui::Frame, area: Rect) {
    let traces = kernel::syscall::get_syscall_traces();
    let mut items = Vec::new();
    
    for trace in traces.iter().rev() {
        let result_label = match &trace.result {
            Ok(val) => format!("SUCCESS (ret: {})", val),
            Err(e) => format!("ERROR: {}", e),
        };
        let log_style = if trace.result.is_ok() { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };

        items.push(ListItem::new(vec![
            Line::from(Span::styled(
                format!("PID {}: {} (arg1: {}, arg2: {}, arg3: {})", trace.pid, trace.syscall_name, trace.args[0], trace.args[1], trace.args[2]),
                Style::default().fg(Color::White)
            )),
            Line::from(Span::styled(format!("  -> Result: {}", result_label), log_style)),
            Line::from(Span::raw("")),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [POSIX Syscall Trace Explorer] ");

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

// Tab 3: Teacher Dashboard
#[cfg(feature = "tui")]
fn draw_teacher_dashboard_tab(f: &mut ratatui::Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Teacher terminal controls
            Constraint::Min(4),     // Announcements scrolling
            Constraint::Length(3),  // Input line
        ])
        .split(area);

    let info_text = vec![
        Line::from(Span::styled("NovaSchool Classroom Dashboard Controls:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("")),
        Line::from("Type one of the following commands in prompt below to test teacher features:"),
        Line::from(vec![
            Span::styled("  lock        ", Style::default().fg(Color::Cyan)),
            Span::raw("Lock student terminal input"),
        ]),
        Line::from(vec![
            Span::styled("  unlock      ", Style::default().fg(Color::Cyan)),
            Span::raw("Unlock student terminal input"),
        ]),
        Line::from(vec![
            Span::styled("  distribute  ", Style::default().fg(Color::Cyan)),
            Span::raw("Distribute 'lab1_handout.txt' to student workspaces"),
        ]),
        Line::from(vec![
            Span::styled("  reset       ", Style::default().fg(Color::Cyan)),
            Span::raw("Auto-Reset / wipe clean student home directories"),
        ]),
        Line::from(vec![
            Span::styled("  msg <txt>   ", Style::default().fg(Color::Cyan)),
            Span::raw("Broadcast classroom announcements directly to VGA output"),
        ]),
    ];

    let controls = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL).title(" [Classroom Management Panel] "));
    f.render_widget(controls, chunks[0]);

    // Announcement lists
    let announcements = classroom::get_announcements();
    let mut list_items = Vec::new();
    for ann in announcements.iter().rev() {
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled("📢 Announcement: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
            Span::raw(ann),
        ])));
    }
    let announcements_list = List::new(list_items).block(Block::default().borders(Borders::ALL).title(" [Recent Class Alerts] "));
    f.render_widget(announcements_list, chunks[1]);

    // Prompt input
    let input = TEACHER_INPUT_BUFFER.lock().unwrap().clone();
    let input_par = Paragraph::new(input)
        .block(Block::default().borders(Borders::ALL).title(" [Teacher Command Prompt] "));
    f.render_widget(input_par, chunks[2]);
}

// Tab 4: Assistant
#[cfg(feature = "tui")]
fn draw_assistant_tab(f: &mut ratatui::Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),  // Chat log
            Constraint::Length(3), // Chat input line
        ])
        .split(area);

    let hist_lock = ASSISTANT_HISTORY.lock().unwrap();
    let mut items = Vec::new();
    if let Some(ref hist) = *hist_lock {
        for (sender, text) in hist {
            let (label, style) = if sender == "user" {
                ("🎓 Student: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                ("🤖 Assistant: ", Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD))
            };
            
            items.push(ListItem::new(vec![
                Line::from(vec![Span::styled(label, style), Span::raw(text)]),
                Line::from(""),
            ]));
        }
    }

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" [Nova School AI Assistant Chatbot] "));
    f.render_widget(list, chunks[0]);

    // Chat prompt input
    let input = ASSISTANT_INPUT_BUFFER.lock().unwrap().clone();
    let input_par = Paragraph::new(input)
        .block(Block::default().borders(Borders::ALL).title(" [Ask Assistant concept (e.g. scheduler, memory, novafs)] "));
    f.render_widget(input_par, chunks[1]);
}

// Tab 5: Security Auditor
#[cfg(feature = "tui")]
fn draw_security_auditor_tab(f: &mut ratatui::Frame, area: Rect) {
    let logs = kernel::security::get_security_audit_logs();
    let mut items = Vec::new();

    for log in logs.iter().rev() {
        let (icon, style) = if log.status == "DENIED" {
            ("❌ [VIOLATION] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else {
            ("🛡️ [AUDIT] ", Style::default().fg(Color::Green))
        };

        items.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(icon, style),
                Span::styled(format!("UID {} | Action: {} | Status: {}", log.user_uid, log.action, log.status), Style::default().fg(Color::White)),
            ]),
            Line::from(Span::styled(format!("  Details: {}", log.details), Style::default().fg(Color::DarkGray))),
            Line::from(""),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [Mandatory Access Control (MAC) & Intrusion Audit Log] ");
    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

#[cfg(feature = "gui-window")]
pub fn start_gui_environment() -> Result<(), std::io::Error> {
    // Spawn background simulation thread to run kernel scheduling / networking
    std::thread::spawn(move || {
        let tick_rate = std::time::Duration::from_millis(150);
        let mut last_tick = std::time::Instant::now();
        loop {
            // Run network card background loops and schedule process increments
            networking::poll_network_card();
            
            // Accumulate CPU ticks on random active process
            let curr_p = kernel::task::get_current_pid();
            kernel::scheduler::increment_task_ticks(curr_p);

            // Periodically trigger a virtual timer interrupt
            if last_tick.elapsed() >= std::time::Duration::from_secs(4) {
                kernel::interrupts::trigger_interrupt_vector(0x20, 0);
                last_tick = std::time::Instant::now();
            }
            
            std::thread::sleep(tick_rate);
        }
    });

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("NovaSchool OS Simulator")
            .with_inner_size([1200.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "NovaSchool OS",
        options,
        Box::new(|_cc| Box::new(gui_app::NovaGuiApp::new())),
    ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(())
}
