use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Cell};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Style, Color, Modifier};
use ratatui::text::{Span, Line};

use kernel::task::{Process, ProcessState};
use kernel::mem::FrameOwner;

// Renders the memory map grid onto a specified area
pub fn render_memory_grid(f: &mut ratatui::Frame, area: Rect, frames: &[FrameOwner]) {
    // Render first 256 physical frames (representing 1MB RAM chunk layout) as a 16x16 grid
    let mut grid_lines = Vec::new();
    
    for row in 0..16 {
        let mut line_spans = Vec::new();
        for col in 0..16 {
            let idx = row * 16 + col;
            let symbol = "■ ";
            let style = match frames.get(idx).unwrap_or(&FrameOwner::Free) {
                FrameOwner::Free => Style::default().fg(Color::DarkGray),
                FrameOwner::Kernel => Style::default().fg(Color::Red),
                FrameOwner::Process(pid) => {
                    let color = match pid % 5 {
                        0 => Color::Green,
                        1 => Color::LightGreen,
                        2 => Color::LightBlue,
                        3 => Color::Magenta,
                        _ => Color::LightCyan,
                    };
                    Style::default().fg(color)
                }
                FrameOwner::Shared => Style::default().fg(Color::Cyan),
                FrameOwner::Cow(_) => Style::default().fg(Color::Yellow),
            };
            line_spans.push(Span::styled(symbol, style));
        }
        grid_lines.push(Line::from(line_spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [Memory Map - First 256 Frames (1MB RAM)] ");
    
    let paragraph = Paragraph::new(grid_lines).block(block);
    f.render_widget(paragraph, area);
}

// Renders the processes list as a Table widget
pub fn render_processes_table(f: &mut ratatui::Frame, area: Rect, processes: &[Process]) {
    let header_cells = ["PID", "NAME", "PRIO", "STATE", "TICKS"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).style(Style::default().bg(Color::Blue)).height(1);

    let rows: Vec<Row> = processes
        .iter()
        .filter(|p| p.state != ProcessState::Killed)
        .map(|p| {
            let pid_cell = Cell::from(p.pid.to_string());
            let name_cell = Cell::from(p.name.clone());
            let prio_cell = Cell::from(format!("{:?}", p.priority));
            let state_cell = Cell::from(format!("{:?}", p.state));
            let ticks_cell = Cell::from(p.cpu_ticks.to_string());

            let row_style = match p.state {
                ProcessState::Running => Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
                ProcessState::Ready => Style::default().fg(Color::White),
                ProcessState::Blocked => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::DarkGray),
            };

            Row::new(vec![pid_cell, name_cell, prio_cell, state_cell, ticks_cell])
                .style(row_style)
                .height(1)
        })
        .collect();

    let table = Table::new(rows, &[
        Constraint::Length(5),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(8),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" [Process Control Block (PCB) Table] "));

    f.render_widget(table, area);
}

// Renders the System Interrupts stats
pub fn render_interrupts(f: &mut ratatui::Frame, area: Rect) {
    let (timer, kbd, pf) = kernel::interrupts::get_interrupt_stats();
    let text = vec![
        Line::from(vec![
            Span::styled("Timer Interrupts (IRQ 0): ", Style::default().fg(Color::White)),
            Span::styled(timer.to_string(), Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Keyboard Interrupts (IRQ 1): ", Style::default().fg(Color::White)),
            Span::styled(kbd.to_string(), Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Page Fault Exceptions (Vector 14): ", Style::default().fg(Color::White)),
            Span::styled(pf.to_string(), Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [System Interrupt Counters] ");
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
