use crate::i18n::t;
use crate::spreadsheet::{auto_output_path, ExportFormat};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportChoice {
    Xlsx,
    Csv,
    Skip,
}

pub struct ExportState {
    pub choice: ExportChoice,
    pub list_state: ListState,
    /// Auto-generated output path (filled after confirm)
    pub output_path: String,
}

impl ExportState {
    pub fn new() -> Self {
        let mut ls = ListState::default();
        ls.select(Some(0));
        Self {
            choice: ExportChoice::Xlsx,
            list_state: ls,
            output_path: String::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => {
                let i = self.list_state.selected().unwrap_or(0);
                let next = if i == 0 { 2 } else { i - 1 };
                self.list_state.select(Some(next));
                self.choice = [ExportChoice::Xlsx, ExportChoice::Csv, ExportChoice::Skip][next].clone();
            }
            KeyCode::Down => {
                let i = self.list_state.selected().unwrap_or(0);
                let next = (i + 1) % 3;
                self.list_state.select(Some(next));
                self.choice = [ExportChoice::Xlsx, ExportChoice::Csv, ExportChoice::Skip][next].clone();
            }
            KeyCode::Enter => {
                if self.choice != ExportChoice::Skip {
                    let fmt = match self.choice {
                        ExportChoice::Xlsx => ExportFormat::Xlsx,
                        ExportChoice::Csv => ExportFormat::Csv,
                        _ => unreachable!(),
                    };
                    self.output_path = auto_output_path(&fmt);
                }
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer = Block::default()
            .title(Span::styled(
                format!(" {} ", t("export_title")),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(area);

        let format_items = vec![
            ListItem::new(format!("  {}", t("export_format_xlsx"))),
            ListItem::new(format!("  {}", t("export_format_csv"))),
            ListItem::new(format!("  {}", t("export_skip"))),
        ];
        let format_list = List::new(format_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(format_list, chunks[0], &mut self.list_state);

        if self.choice != ExportChoice::Skip {
            let preview_ext = match self.choice {
                ExportChoice::Xlsx => "xlsx",
                ExportChoice::Csv => "csv",
                _ => "",
            };
            let hint = Paragraph::new(Line::from(vec![
                Span::styled(t("export_path_label") + ": ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("packages_report_<timestamp>.{}", preview_ext),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            frame.render_widget(hint, chunks[1]);
        }

        let help = Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Confirm"),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }
}
