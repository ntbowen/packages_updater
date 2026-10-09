use crate::config::RepoMode;
use crate::i18n::t;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct RepoSelectState {
    pub title_key: String,
    pub mode: RepoMode,
    pub path: String,
    pub mode_list_state: ListState,
    pub focused_field: Field,
    pub cursor_pos: usize,
}

#[derive(PartialEq)]
pub enum Field {
    ModeList,
    PathInput,
}

impl RepoSelectState {
    pub fn new(title_key: &str) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            title_key: title_key.to_string(),
            mode: RepoMode::Local,
            path: String::new(),
            mode_list_state: state,
            focused_field: Field::ModeList,
            cursor_pos: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match &self.focused_field {
            Field::ModeList => match key.code {
                KeyCode::Up => {
                    self.mode = RepoMode::Local;
                    self.mode_list_state.select(Some(0));
                }
                KeyCode::Down => {
                    self.mode = RepoMode::Remote;
                    self.mode_list_state.select(Some(1));
                }
                KeyCode::Tab | KeyCode::Enter => {
                    self.focused_field = Field::PathInput;
                }
                _ => {}
            },
            Field::PathInput => match key.code {
                KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
                    self.path.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                }
                KeyCode::Backspace => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                        self.path.remove(self.cursor_pos);
                    }
                }
                KeyCode::Left => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.cursor_pos < self.path.len() {
                        self.cursor_pos += 1;
                    }
                }
                KeyCode::Tab => {
                    self.focused_field = Field::ModeList;
                }
                KeyCode::Enter => {
                    if !self.path.is_empty() {
                        return true;
                    }
                }
                _ => {}
            },
        }
        false
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer = Block::default()
            .title(Span::styled(
                format!(" {} ", t(&self.title_key)),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(area);

        let mode_label = Paragraph::new(t("repo_mode_local") + " / " + &t("repo_mode_remote"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(mode_label, chunks[0]);

        let mode_items = vec![
            ListItem::new(format!("  {}", t("repo_mode_local"))),
            ListItem::new(format!("  {}", t("repo_mode_remote"))),
        ];
        let mode_border_style = if self.focused_field == Field::ModeList {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let mode_list = List::new(mode_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(mode_border_style)
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(mode_list, chunks[1], &mut self.mode_list_state);

        let path_label = Paragraph::new(t("repo_path_label"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(path_label, chunks[2]);

        let path_border_style = if self.focused_field == Field::PathInput {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let hint = t("repo_browse_hint");
        let display_text = if self.path.is_empty() {
            Span::styled(hint.as_str(), Style::default().fg(Color::DarkGray))
        } else {
            Span::raw(self.path.as_str())
        };
        let path_widget = Paragraph::new(Line::from(display_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(path_border_style)
                    .border_type(BorderType::Rounded),
            );
        frame.render_widget(path_widget, chunks[3]);

        if self.focused_field == Field::PathInput {
            frame.set_cursor_position((
                chunks[3].x + 1 + self.cursor_pos as u16,
                chunks[3].y + 1,
            ));
        }

        let help = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch field  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Confirm"),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[5]);
    }
}
