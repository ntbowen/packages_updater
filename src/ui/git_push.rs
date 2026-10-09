use crate::i18n::t;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushMethod {
    LocalCredentials,
    Token,
    Skip,
}

pub struct GitPushState {
    pub method: PushMethod,
    pub token: String,
    pub branch: String,
    pub origin_url: String,
    pub list_state: ListState,
    pub focused: PushFocus,
    pub token_cursor: usize,
    pub branch_cursor: usize,
    pub status_msg: String,
    pub push_logs: Vec<String>,
    pub done: bool,
    pub pushing: bool,
}

#[derive(PartialEq)]
pub enum PushFocus {
    Method,
    Token,
    Branch,
}

impl GitPushState {
    pub fn new(branch: &str, origin_url: &str) -> Self {
        let mut ls = ListState::default();
        ls.select(Some(0));
        Self {
            method: PushMethod::LocalCredentials,
            token: String::new(),
            branch: branch.to_string(),
            origin_url: origin_url.to_string(),
            list_state: ls,
            focused: PushFocus::Method,
            token_cursor: 0,
            branch_cursor: branch.len(),
            status_msg: String::new(),
            push_logs: Vec::new(),
            done: false,
            pushing: false,
        }
    }

    pub fn add_log(&mut self, msg: String) {
        self.push_logs.push(msg);
        if self.push_logs.len() > 100 {
            self.push_logs.remove(0);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match &self.focused {
            PushFocus::Method => match key.code {
                KeyCode::Up => {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = if i == 0 { 2 } else { i - 1 };
                    self.list_state.select(Some(next));
                    self.method = [PushMethod::LocalCredentials, PushMethod::Token, PushMethod::Skip][next].clone();
                }
                KeyCode::Down => {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = (i + 1) % 3;
                    self.list_state.select(Some(next));
                    self.method = [PushMethod::LocalCredentials, PushMethod::Token, PushMethod::Skip][next].clone();
                }
                KeyCode::Tab => {
                    self.focused = PushFocus::Branch;
                }
                KeyCode::Enter => {
                    if self.method == PushMethod::Token {
                        self.focused = PushFocus::Token;
                    } else {
                        return true;
                    }
                }
                _ => {}
            },
            PushFocus::Token => {
                self.handle_text_input(key, true);
                if key.code == KeyCode::Tab {
                    self.focused = PushFocus::Branch;
                } else if key.code == KeyCode::Enter && !self.token.is_empty() {
                    return true;
                }
            }
            PushFocus::Branch => {
                self.handle_text_input(key, false);
                if key.code == KeyCode::Tab {
                    self.focused = PushFocus::Method;
                } else if key.code == KeyCode::Enter && !self.branch.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    fn handle_text_input(&mut self, key: KeyEvent, is_token: bool) {
        let (text, cursor) = if is_token {
            (&mut self.token, &mut self.token_cursor)
        } else {
            (&mut self.branch, &mut self.branch_cursor)
        };
        match key.code {
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
                text.insert(*cursor, c);
                *cursor += 1;
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    *cursor -= 1;
                    text.remove(*cursor);
                }
            }
            KeyCode::Left => {
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
            KeyCode::Right => {
                if *cursor < text.len() {
                    *cursor += 1;
                }
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer = Block::default()
            .title(Span::styled(
                format!(" {} ", t("push_title")),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(1), // origin url
                Constraint::Length(5), // method list
                Constraint::Length(1), // token label
                Constraint::Length(3), // token input
                Constraint::Length(1), // branch label
                Constraint::Length(3), // branch input
                Constraint::Min(3),    // push logs
                Constraint::Length(1), // status
                Constraint::Length(2), // help
            ])
            .split(area);

        // Origin URL info
        let url_color = if self.origin_url.starts_with("(no") { Color::Red } else { Color::DarkGray };
        let origin_line = Paragraph::new(Line::from(vec![
            Span::styled("Remote: ", Style::default().fg(Color::Gray)),
            Span::styled(self.origin_url.as_str(), Style::default().fg(url_color)),
        ]));
        frame.render_widget(origin_line, chunks[0]);

        let method_items = vec![
            ListItem::new(format!("  {}", t("push_method_local"))),
            ListItem::new(format!("  {}", t("push_method_token"))),
            ListItem::new(format!("  {}", t("push_skip"))),
        ];
        let method_border = if self.focused == PushFocus::Method && !self.pushing {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let method_list = List::new(method_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(method_border)
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(method_list, chunks[1], &mut self.list_state);

        if self.method == PushMethod::Token {
            let token_label = Paragraph::new(t("push_token_label"))
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(token_label, chunks[2]);

            let token_border = if self.focused == PushFocus::Token {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let masked: String = "*".repeat(self.token.len());
            let token_widget = Paragraph::new(Line::from(masked.as_str()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(token_border)
                        .border_type(BorderType::Rounded),
                );
            frame.render_widget(token_widget, chunks[3]);

            if self.focused == PushFocus::Token {
                frame.set_cursor_position((
                    chunks[3].x + 1 + self.token_cursor as u16,
                    chunks[3].y + 1,
                ));
            }
        }

        let branch_label = Paragraph::new(t("push_branch_label"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(branch_label, chunks[4]);

        let branch_border = if self.focused == PushFocus::Branch {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let branch_widget = Paragraph::new(Line::from(self.branch.as_str()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(branch_border)
                    .border_type(BorderType::Rounded),
            );
        frame.render_widget(branch_widget, chunks[5]);

        if self.focused == PushFocus::Branch {
            frame.set_cursor_position((
                chunks[5].x + 1 + self.branch_cursor as u16,
                chunks[5].y + 1,
            ));
        }

        // Push log area
        if !self.push_logs.is_empty() || self.pushing {
            let log_area = chunks[6];
            let visible = log_area.height.saturating_sub(2) as usize;
            let start = self.push_logs.len().saturating_sub(visible);
            let items: Vec<ListItem> = self.push_logs[start..]
                .iter()
                .map(|l| {
                    let color = if l.starts_with("✓") { Color::Green }
                        else if l.starts_with("✗") || l.to_lowercase().contains("error") { Color::Red }
                        else { Color::DarkGray };
                    ListItem::new(Span::styled(l.as_str(), Style::default().fg(color)))
                })
                .collect();
            let log_list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .border_type(BorderType::Rounded)
                    .title(Span::styled(" Push Log ", Style::default().fg(Color::DarkGray))),
            );
            frame.render_widget(log_list, log_area);
        }

        if !self.status_msg.is_empty() {
            let color = if self.done { Color::Green } else if self.pushing { Color::Yellow } else { Color::Red };
            let status = Paragraph::new(Line::from(Span::styled(
                self.status_msg.as_str(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(status, chunks[7]);
        }

        let help = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Push"),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[8]);
    }
}
