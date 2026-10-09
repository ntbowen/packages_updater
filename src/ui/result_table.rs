use crate::comparator::{Action, CompareResult, CompareStatus};
use crate::i18n::t;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Row, ScrollbarState, Table, TableState, Tabs,
    },
    Frame,
};

pub struct ResultTableState {
    pub results: Vec<CompareResult>,
    pub tab: usize,
    pub table_state: TableState,
    pub scroll_state: ScrollbarState,
}

const TABS: [&str; 5] = [
    "status_upstream_newer",
    "status_upstream_only",
    "status_mine_only",
    "status_same",
    "status_manual",
];

fn status_for_tab(tab: usize) -> CompareStatus {
    match tab {
        0 => CompareStatus::UpstreamNewer,
        1 => CompareStatus::UpstreamOnly,
        2 => CompareStatus::MineOnly,
        3 => CompareStatus::Same,
        _ => CompareStatus::Manual,
    }
}

impl ResultTableState {
    pub fn new(results: Vec<CompareResult>) -> Self {
        let mut ts = TableState::default();
        ts.select(Some(0));
        let len = results.len();
        Self {
            results,
            tab: 0,
            table_state: ts,
            scroll_state: ScrollbarState::new(len),
        }
    }

    fn current_results(&self) -> Vec<&CompareResult> {
        let status = status_for_tab(self.tab);
        self.results.iter().filter(|r| r.status == status).collect()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let rows = self.current_results().len();
        match key.code {
            KeyCode::Tab => {
                self.tab = (self.tab + 1) % TABS.len();
                self.table_state.select(Some(0));
            }
            KeyCode::BackTab => {
                self.tab = if self.tab == 0 { TABS.len() - 1 } else { self.tab - 1 };
                self.table_state.select(Some(0));
            }
            KeyCode::Up => {
                let i = self.table_state.selected().unwrap_or(0);
                self.table_state.select(Some(if i == 0 { rows.saturating_sub(1) } else { i - 1 }));
            }
            KeyCode::Down => {
                let i = self.table_state.selected().unwrap_or(0);
                if rows > 0 {
                    self.table_state.select(Some((i + 1) % rows));
                }
            }
            KeyCode::Char(' ') => {
                if self.tab == 4 {
                    let i = self.table_state.selected().unwrap_or(0);
                    let status = status_for_tab(self.tab);
                    let mut count = 0;
                    for result in &mut self.results {
                        if result.status == status {
                            if count == i {
                                result.selected = !result.selected;
                                result.action = if result.selected {
                                    Action::Overwrite
                                } else {
                                    Action::Skip
                                };
                                break;
                            }
                            count += 1;
                        }
                    }
                }
            }
            KeyCode::Char('a') => {
                if self.tab == 4 {
                    for result in &mut self.results {
                        if result.status == CompareStatus::Manual {
                            result.selected = true;
                            result.action = Action::Overwrite;
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.tab == 4 {
                    for result in &mut self.results {
                        if result.status == CompareStatus::Manual {
                            result.selected = false;
                            result.action = Action::Skip;
                        }
                    }
                }
            }
            KeyCode::Enter => return true,
            _ => {}
        }
        false
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let tab_titles: Vec<Line> = TABS
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let status = status_for_tab(i);
                let count = self.results.iter().filter(|r| r.status == status).count();
                Line::from(format!("{} ({})", t(key), count))
            })
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(2)])
            .split(area);

        let tabs = Tabs::new(tab_titles)
            .select(self.tab)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
            .divider("|");
        frame.render_widget(tabs, chunks[0]);

        let header_cells = [
            t("col_name"),
            t("col_mine_ver"),
            t("col_upstream_ver"),
            t("col_action"),
            t("col_path"),
        ]
        .iter()
        .map(|h| Cell::from(h.clone()).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .collect::<Vec<_>>();
        let header = Row::new(header_cells).height(1).bottom_margin(0);

        let current = self.current_results();
        let rows: Vec<Row> = current
            .iter()
            .enumerate()
            .map(|(_i, r)| {
                let selected = r.selected;
                let style = if self.tab == 4 && selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                let check = if self.tab == 4 {
                    if selected { "✓ " } else { "  " }.to_string()
                } else {
                    String::new()
                };
                let action_str = match r.action {
                    Action::Overwrite => t("action_overwrite"),
                    Action::Add => t("action_add"),
                    Action::Keep => t("action_keep"),
                    Action::Skip => t("action_skip"),
                };
                let path = if r.mine.is_some() { r.my_path() } else { r.upstream_path() };
                Row::new(vec![
                    Cell::from(format!("{}{}", check, r.name)),
                    Cell::from(r.my_version().to_string()),
                    Cell::from(r.upstream_version().to_string()),
                    Cell::from(action_str),
                    Cell::from(path.to_string()),
                ])
                .style(style)
                .height(1)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(22),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(12),
                Constraint::Percentage(38),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));
        frame.render_stateful_widget(table, chunks[1], &mut self.table_state);

        let help = if self.tab == 4 {
            Line::from(vec![
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::raw(" Toggle  "),
                Span::styled("a", Style::default().fg(Color::Green)),
                Span::raw(" SelectAll  "),
                Span::styled("d", Style::default().fg(Color::Red)),
                Span::raw(" DeselectAll  "),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" Switch tab  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Next"),
            ])
        } else {
            Line::from(vec![
                Span::styled("↑↓", Style::default().fg(Color::Yellow)),
                Span::raw(" Navigate  "),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" Switch tab  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Next"),
            ])
        };

        let help_widget = ratatui::widgets::Paragraph::new(help)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(help_widget, chunks[2]);
    }
}
