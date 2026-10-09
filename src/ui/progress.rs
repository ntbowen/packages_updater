use crate::i18n::t;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

const MAX_LOG: usize = 200;

pub struct ProgressState {
    pub total: usize,
    pub done: usize,
    pub message: String,
    pub logs: Vec<String>,
    pub finished: bool,
    pub error: Option<String>,
}

impl ProgressState {
    pub fn new() -> Self {
        Self {
            total: 0,
            done: 0,
            message: String::new(),
            logs: Vec::new(),
            finished: false,
            error: None,
        }
    }

    pub fn push_log(&mut self, msg: String) {
        self.message = msg.clone();
        self.logs.push(msg);
        if self.logs.len() > MAX_LOG {
            self.logs.remove(0);
        }
    }

    pub fn percent(&self) -> u16 {
        if self.total == 0 {
            return 0;
        }
        let pct = ((self.done as f64 / self.total as f64) * 100.0) as u16;
        pct.min(99)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let title = t("step_scanning");
        let outer = Block::default()
            .title(Span::styled(
                format!(" {} ", title),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(area);

        let status_text = if let Some(err) = &self.error {
            Span::styled(format!("✗ {}", err), Style::default().fg(Color::Red))
        } else if self.finished {
            Span::styled(
                format!("✓ {}  ({} packages)", t("scan_done_short"), self.done),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!("⟳ {}  [{}/{}]", self.message, self.done, self.total),
                Style::default().fg(Color::Cyan),
            )
        };
        let status = Paragraph::new(Line::from(status_text)).alignment(Alignment::Left);
        frame.render_widget(status, chunks[0]);

        let pct = if self.finished { 100 } else { self.percent() };
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pct);
        frame.render_widget(gauge, chunks[1]);

        let log_area = chunks[2];
        let visible = log_area.height as usize;
        let start = self.logs.len().saturating_sub(visible);
        let items: Vec<ListItem> = self.logs[start..]
            .iter()
            .map(|l| {
                let color = if l.starts_with("✗") {
                    Color::Red
                } else if l.starts_with("✓") || l.starts_with("OK") {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                ListItem::new(Span::styled(l.as_str(), Style::default().fg(color)))
            })
            .collect();
        let log_list = List::new(items).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(" Log ", Style::default().fg(Color::DarkGray))),
        );
        frame.render_widget(log_list, log_area);
    }
}
