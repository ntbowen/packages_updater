use crate::i18n::{available_locales, set_locale, t};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct LangSelectState {
    pub list_state: ListState,
}

impl LangSelectState {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { list_state: state }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let locales = available_locales();
        match key.code {
            KeyCode::Up => {
                let i = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(if i == 0 { locales.len() - 1 } else { i - 1 }));
            }
            KeyCode::Down => {
                let i = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some((i + 1) % locales.len()));
            }
            KeyCode::Enter => {
                let i = self.list_state.selected().unwrap_or(0);
                let (code, _) = locales[i];
                set_locale(code);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let locales = available_locales();
        let items: Vec<ListItem> = locales
            .iter()
            .map(|(_, label)| {
                ListItem::new(Line::from(Span::raw(*label)))
            })
            .collect();

        let outer = Block::default()
            .title(Span::styled(
                format!(" {} ", t("step_lang")),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Length(4), Constraint::Min(4), Constraint::Length(2)])
            .split(area);

        frame.render_widget(outer, area);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Select"),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(hint, chunks[0]);

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);
    }
}
