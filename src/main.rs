mod i18n;
mod config;
mod makefile;
mod version;
mod scanner;
mod comparator;
mod patcher;
mod spreadsheet;
mod git_ops;
mod persist;
mod ui;

use anyhow::Result;
use comparator::{compare_repos, Action, CompareResult};
use config::{AppConfig, RepoMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git_ops::AuthMethod;
use i18n::{init_translations, set_locale, t, tf};
use persist::{load_config, save_config, SavedConfig};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use scanner::{scan_local_repo, scan_remote_repo};
use spreadsheet::{export, ExportFormat};
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use ui::{
    export_select::{ExportChoice, ExportState},
    git_push::{GitPushState, PushMethod},
    lang_select::LangSelectState,
    progress::ProgressState,
    repo_select::RepoSelectState,
    result_table::ResultTableState,
};

enum ScanMsg {
    Log(String),
    Total(usize),
    Done(usize),
    Finished(Vec<CompareResult>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
enum AppStep {
    LangSelect,
    MineRepo,
    UpstreamRepo,
    Scanning,
    Results,
    Export,
    Apply,
    GitPush,
    Done,
}

struct App {
    step: AppStep,
    config: AppConfig,
    lang_state: LangSelectState,
    mine_repo_state: RepoSelectState,
    upstream_repo_state: RepoSelectState,
    progress_state: Option<ProgressState>,
    result_state: Option<ResultTableState>,
    export_state: ExportState,
    git_push_state: Option<GitPushState>,
    scan_rx: Option<Receiver<ScanMsg>>,
    push_rx: Option<Receiver<Result<String>>>,
    status_message: String,
    error_message: String,
}

impl App {
    fn new() -> Self {
        Self {
            step: AppStep::LangSelect,
            config: AppConfig::default(),
            lang_state: LangSelectState::new(),
            mine_repo_state: RepoSelectState::new("step_mine_repo"),
            upstream_repo_state: RepoSelectState::new("step_upstream_repo"),
            progress_state: None,
            result_state: None,
            export_state: ExportState::new(),
            git_push_state: None,
            scan_rx: None,
            push_rx: None,
            status_message: String::new(),
            error_message: String::new(),
        }
    }

    fn reset_for_new_task(&mut self) {
        self.step = AppStep::MineRepo;
        self.progress_state = None;
        self.result_state = None;
        self.export_state = ExportState::new();
        self.git_push_state = None;
        self.scan_rx = None;
        self.push_rx = None;
        self.status_message.clear();
        self.error_message.clear();
    }
}

fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    let outer = Block::default()
        .title(Span::styled(
            format!(" {} ", t("app_title")),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    draw_steps_bar(app, frame, chunks[0]);

    match &app.step {
        AppStep::LangSelect => app.lang_state.render(frame, chunks[1]),
        AppStep::MineRepo => app.mine_repo_state.render(frame, chunks[1]),
        AppStep::UpstreamRepo => app.upstream_repo_state.render(frame, chunks[1]),
        AppStep::Scanning => {
            if let Some(ps) = &app.progress_state {
                ps.render(frame, chunks[1]);
            }
        }
        AppStep::Results => {
            if let Some(rs) = &mut app.result_state {
                rs.render(frame, chunks[1]);
            }
        }
        AppStep::Export => {
            app.export_state.render(frame, chunks[1]);
        }
        AppStep::Apply => {
            let msg = if app.error_message.is_empty() {
                app.status_message.as_str()
            } else {
                app.error_message.as_str()
            };
            let color = if app.error_message.is_empty() {
                Color::Green
            } else {
                Color::Red
            };
            let help = format!("{}\n\n[Enter] {}", msg, t("btn_confirm"));
            let p = Paragraph::new(help)
                .alignment(Alignment::Center)
                .style(Style::default().fg(color))
                .block(
                    Block::default()
                        .title(Span::styled(
                            format!(" {} ", t("apply_title")),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );
            frame.render_widget(p, chunks[1]);
        }
        AppStep::GitPush => {
            if let Some(gp) = &mut app.git_push_state {
                gp.render(frame, chunks[1]);
            }
        }
        AppStep::Done => {
            let lines = vec![
                Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        app.status_message.as_str(),
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[Enter] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(t("done_new_task"), Style::default().fg(Color::Cyan)),
                    Span::raw("    "),
                    Span::styled("[q] ", Style::default().fg(Color::DarkGray)),
                    Span::styled(t("done_quit"), Style::default().fg(Color::DarkGray)),
                ]),
            ];
            let p = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!(" {} ", t("done_title")),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
            frame.render_widget(p, chunks[1]);
        }
    }

    let quit_help = Paragraph::new(Line::from(vec![
        Span::styled("Ctrl+C", Style::default().fg(Color::DarkGray)),
        Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Right);
    frame.render_widget(quit_help, chunks[2]);
}

fn draw_steps_bar(app: &App, frame: &mut Frame, area: Rect) {
    let steps = [
        ("step_lang", AppStep::LangSelect),
        ("step_mine_repo", AppStep::MineRepo),
        ("step_upstream_repo", AppStep::UpstreamRepo),
        ("step_scanning", AppStep::Scanning),
        ("step_results", AppStep::Results),
        ("step_export", AppStep::Export),
        ("step_apply", AppStep::Apply),
        ("step_push", AppStep::GitPush),
    ];

    let constraints: Vec<Constraint> = steps
        .iter()
        .map(|_| Constraint::Ratio(1, steps.len() as u32))
        .collect();
    let step_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let current_idx = steps
        .iter()
        .position(|(_, s)| s == &app.step);
    let all_done = app.step == AppStep::Done;

    for (i, ((key, _step), step_area)) in steps.iter().zip(step_areas.iter()).enumerate() {
        let (fg, bg, modifier, prefix) = if all_done {
            (Color::Green, Color::Reset, Modifier::empty(), "✓ ")
        } else if Some(i) == current_idx {
            (Color::Black, Color::Cyan, Modifier::BOLD, "")
        } else if current_idx.is_some() && i < current_idx.unwrap() {
            (Color::Green, Color::Reset, Modifier::empty(), "✓ ")
        } else {
            (Color::DarkGray, Color::Reset, Modifier::empty(), "")
        };
        let label = format!("{}{}", prefix, t(key));
        let p = Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(fg).bg(bg).add_modifier(modifier));
        frame.render_widget(p, *step_area);
    }
}

fn start_scan(app: &mut App) {
    app.progress_state = Some(ProgressState::new());
    let (tx, rx) = mpsc::channel::<ScanMsg>();
    app.scan_rx = Some(rx);

    let mine_path = app.config.mine_path.clone();
    let mine_mode = app.config.mine_mode.clone();
    let upstream_path = app.config.upstream_path.clone();
    let upstream_mode = app.config.upstream_mode.clone();

    std::thread::spawn(move || {
        let tx2 = tx.clone();
        let cb_mine = move |msg: String| { let _ = tx2.send(ScanMsg::Log(format!("[mine] {}", msg))); };
        let tx3 = tx.clone();
        let cb_up   = move |msg: String| { let _ = tx3.send(ScanMsg::Log(format!("[upstream] {}", msg))); };

        let _ = tx.send(ScanMsg::Log(format!("Scanning my repo: {}", mine_path)));
        let mine_result = if mine_mode == RepoMode::Remote {
            scan_remote_repo(&mine_path, Some(&cb_mine))
        } else {
            scan_local_repo(Path::new(&mine_path), Some(&cb_mine))
        };
        let mine_packages = match mine_result {
            Ok(p) => p,
            Err(e) => { let _ = tx.send(ScanMsg::Error(e.to_string())); return; }
        };
        let mine_count = mine_packages.packages.len();
        let _ = tx.send(ScanMsg::Log(format!("My repo: {} packages found", mine_count)));

        let _ = tx.send(ScanMsg::Log(format!("Scanning upstream repo: {}", upstream_path)));
        let up_result = if upstream_mode == RepoMode::Remote {
            scan_remote_repo(&upstream_path, Some(&cb_up))
        } else {
            scan_local_repo(Path::new(&upstream_path), Some(&cb_up))
        };
        let upstream_packages = match up_result {
            Ok(p) => p,
            Err(e) => { let _ = tx.send(ScanMsg::Error(e.to_string())); return; }
        };
        let up_count = upstream_packages.packages.len();
        let _ = tx.send(ScanMsg::Log(format!("Upstream: {} packages found", up_count)));
        let _ = tx.send(ScanMsg::Total(mine_count + up_count));
        let _ = tx.send(ScanMsg::Done(mine_count + up_count));

        let _ = tx.send(ScanMsg::Log("Comparing...".to_string()));
        let results = compare_repos(&mine_packages, &upstream_packages);
        let _ = tx.send(ScanMsg::Log(format!("Done. {} entries.", results.len())));
        let _ = tx.send(ScanMsg::Finished(results));
    });
}

fn do_apply(app: &mut App) -> Result<usize> {
    let results = app
        .result_state
        .as_ref()
        .map(|rs| rs.results.clone())
        .unwrap_or_default();

    let mine_path = Path::new(&app.config.mine_path).to_path_buf();
    let upstream_path = Path::new(&app.config.upstream_path).to_path_buf();

    let count = patcher::apply_changes(&results, &mine_path, &upstream_path)?;
    Ok(count)
}

fn start_push(app: &mut App) {
    let gp = match &app.git_push_state {
        Some(g) => g,
        None => return,
    };
    if gp.method == PushMethod::Skip {
        app.step = AppStep::Done;
        return;
    }

    let method_kind = gp.method.clone();
    let token_val   = gp.token.clone();
    let branch_val  = gp.branch.clone();
    let mine_path   = Path::new(&app.config.mine_path).to_path_buf();

    let count = app
        .result_state
        .as_ref()
        .map(|rs| rs.results.iter().filter(|r| r.action == Action::Overwrite || r.action == Action::Add).count())
        .unwrap_or(0);
    let commit_msg = tf("push_commit_msg", &[("count", &count.to_string())]);

    let (tx, rx) = mpsc::channel::<Result<String>>();
    app.push_rx = Some(rx);
    if let Some(gp) = &mut app.git_push_state {
        gp.pushing = true;
        gp.status_msg = t("push_pushing");
    }

    std::thread::spawn(move || {
        let tx2 = tx.clone();
        let log_fn = move |msg: String| { let _ = tx2.send(Ok(msg)); };

        let method = match method_kind {
            PushMethod::LocalCredentials => AuthMethod::LocalCredentials,
            PushMethod::Token => AuthMethod::Token(token_val),
            PushMethod::Skip => unreachable!(),
        };

        match git_ops::commit_and_push(&mine_path, &commit_msg, &branch_val, &method, &log_fn) {
            Ok(()) => { let _ = tx.send(Ok("__DONE__".to_string())); }
            Err(e) => { let _ = tx.send(Err(e)); }
        }
    });
}

fn main() -> Result<()> {
    init_translations();

    let saved = load_config();
    // Pre-set locale from saved config
    if !saved.locale.is_empty() {
        set_locale(&saved.locale);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    // Restore saved config into app state
    {
        if !saved.locale.is_empty() {
            app.config.locale = saved.locale.clone();
        }
        if !saved.mine_path.is_empty() {
            app.mine_repo_state.path = saved.mine_path.clone();
            app.mine_repo_state.cursor_pos = saved.mine_path.len();
            app.config.mine_path = saved.mine_path.clone();
            app.config.mine_mode = if saved.mine_mode == "remote" { RepoMode::Remote } else { RepoMode::Local };
            let idx = if app.config.mine_mode == RepoMode::Remote { 1 } else { 0 };
            app.mine_repo_state.mode_list_state.select(Some(idx));
            app.mine_repo_state.mode = app.config.mine_mode.clone();
        }
        if !saved.upstream_path.is_empty() {
            app.upstream_repo_state.path = saved.upstream_path.clone();
            app.upstream_repo_state.cursor_pos = saved.upstream_path.len();
            app.config.upstream_path = saved.upstream_path.clone();
            app.config.upstream_mode = if saved.upstream_mode == "remote" { RepoMode::Remote } else { RepoMode::Local };
            let idx = if app.config.upstream_mode == RepoMode::Remote { 1 } else { 0 };
            app.upstream_repo_state.mode_list_state.select(Some(idx));
            app.upstream_repo_state.mode = app.config.upstream_mode.clone();
        }
    }
    loop {
        // Drain scan messages before drawing so the UI reflects latest state
        if app.step == AppStep::Scanning {
            let mut finished = false;
            if let Some(rx) = &app.scan_rx {
                loop {
                    match rx.try_recv() {
                        Ok(ScanMsg::Log(msg)) => {
                            if let Some(ps) = &mut app.progress_state {
                                ps.push_log(msg);
                                ps.done = ps.done.saturating_add(1);
                                if ps.total == 0 { ps.total = 1; }
                            }
                        }
                        Ok(ScanMsg::Total(n)) => {
                            if let Some(ps) = &mut app.progress_state { ps.total = n; }
                        }
                        Ok(ScanMsg::Done(n)) => {
                            if let Some(ps) = &mut app.progress_state { ps.done = n; }
                        }
                        Ok(ScanMsg::Finished(results)) => {
                            if let Some(ps) = &mut app.progress_state {
                                ps.finished = true;
                            }
                            app.result_state = Some(ResultTableState::new(results));
                            finished = true;
                            break;
                        }
                        Ok(ScanMsg::Error(e)) => {
                            if let Some(ps) = &mut app.progress_state {
                                ps.error = Some(e.clone());
                            }
                            app.error_message = e;
                            app.scan_rx = None;
                            // Brief pause so user sees error, then back
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            app.step = AppStep::MineRepo;
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            // Thread ended without Finished — treat as error
                            if app.result_state.is_none() {
                                app.error_message = "Scan thread exited unexpectedly".to_string();
                                app.step = AppStep::MineRepo;
                            }
                            break;
                        }
                    }
                }
            }
            if finished {
                app.scan_rx = None;
                app.step = AppStep::Results;
            }
        }

        // Drain push log messages
        if app.step == AppStep::GitPush {
            if let Some(rx) = &app.push_rx {
                loop {
                    match rx.try_recv() {
                        Ok(Ok(msg)) if msg == "__DONE__" => {
                            if let Some(gp) = &mut app.git_push_state {
                                gp.pushing = false;
                                gp.done = true;
                                gp.status_msg = t("push_done");
                            }
                            app.status_message = t("push_done");
                            app.push_rx = None;
                            app.step = AppStep::Done;
                            break;
                        }
                        Ok(Ok(msg)) => {
                            if let Some(gp) = &mut app.git_push_state {
                                gp.add_log(msg);
                            }
                        }
                        Ok(Err(e)) => {
                            if let Some(gp) = &mut app.git_push_state {
                                gp.pushing = false;
                                gp.done = false;
                                gp.status_msg = tf("push_error", &[("err", &e.to_string())]);
                                gp.add_log(format!("✗ {}", e));
                            }
                            app.push_rx = None;
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            app.push_rx = None;
                            break;
                        }
                    }
                }
            }
        }

        terminal.draw(|f| draw(&mut app, f))?;

        let poll_ms = if app.step == AppStep::Scanning || app.step == AppStep::GitPush { 30 } else { 100 };
        if event::poll(std::time::Duration::from_millis(poll_ms))? {
            match event::read()? {
            Event::Mouse(_) => {}
            Event::Key(key) => {
                if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                    break;
                }

                match &app.step {
                    AppStep::LangSelect => {
                        if app.lang_state.handle_key(key) {
                            app.config.locale = i18n::get_locale().to_string();
                            let _ = save_config(&SavedConfig {
                                locale: app.config.locale.clone(),
                                mine_path: app.config.mine_path.clone(),
                                mine_mode: "local".into(),
                                upstream_path: app.config.upstream_path.clone(),
                                upstream_mode: "local".into(),
                            });
                            app.step = AppStep::MineRepo;
                        }
                    }
                    AppStep::MineRepo => {
                        if app.mine_repo_state.handle_key(key) {
                            app.config.mine_mode = app.mine_repo_state.mode.clone();
                            app.config.mine_path = app.mine_repo_state.path.clone();
                            let _ = save_config(&SavedConfig {
                                locale: app.config.locale.clone(),
                                mine_path: app.config.mine_path.clone(),
                                mine_mode: if app.config.mine_mode == RepoMode::Remote { "remote".into() } else { "local".into() },
                                upstream_path: app.config.upstream_path.clone(),
                                upstream_mode: if app.config.upstream_mode == RepoMode::Remote { "remote".into() } else { "local".into() },
                            });
                            app.step = AppStep::UpstreamRepo;
                        }
                    }
                    AppStep::UpstreamRepo => {
                        if app.upstream_repo_state.handle_key(key) {
                            app.config.upstream_mode = app.upstream_repo_state.mode.clone();
                            app.config.upstream_path = app.upstream_repo_state.path.clone();
                            let _ = save_config(&SavedConfig {
                                locale: app.config.locale.clone(),
                                mine_path: app.config.mine_path.clone(),
                                mine_mode: if app.config.mine_mode == RepoMode::Remote { "remote".into() } else { "local".into() },
                                upstream_path: app.config.upstream_path.clone(),
                                upstream_mode: if app.config.upstream_mode == RepoMode::Remote { "remote".into() } else { "local".into() },
                            });
                            app.step = AppStep::Scanning;
                            start_scan(&mut app);
                        }
                    }
                    AppStep::Scanning => {}
                    AppStep::Results => {
                        if let Some(rs) = &mut app.result_state {
                            if rs.handle_key(key) {
                                app.step = AppStep::Export;
                            }
                        }
                    }
                    AppStep::Export => {
                        if app.export_state.handle_key(key) {
                            let choice = app.export_state.choice.clone();
                            if choice != ExportChoice::Skip {
                                let path = app.export_state.output_path.clone();
                                let format = match choice {
                                    ExportChoice::Xlsx => ExportFormat::Xlsx,
                                    ExportChoice::Csv => ExportFormat::Csv,
                                    _ => unreachable!(),
                                };
                                if let Some(rs) = &app.result_state {
                                    match export(&rs.results, &path, format) {
                                        Ok(_) => {
                                            app.status_message =
                                                tf("export_done", &[("path", &path)])
                                        }
                                        Err(e) => {
                                            app.error_message =
                                                tf("export_error", &[("err", &e.to_string())])
                                        }
                                    }
                                }
                            }
                            let count = app
                                .result_state
                                .as_ref()
                                .map(|rs| {
                                    let overwrite = rs
                                        .results
                                        .iter()
                                        .filter(|r| r.status == comparator::CompareStatus::UpstreamNewer)
                                        .count();
                                    let add = rs
                                        .results
                                        .iter()
                                        .filter(|r| r.status == comparator::CompareStatus::UpstreamOnly)
                                        .count();
                                    (overwrite, add)
                                })
                                .unwrap_or((0, 0));
                            let summary = tf(
                                "apply_summary",
                                &[
                                    ("overwrite", &count.0.to_string()),
                                    ("add", &count.1.to_string()),
                                ],
                            );
                            app.status_message = if app.status_message.is_empty() {
                                summary
                            } else {
                                format!("{}\n{}", app.status_message, summary)
                            };
                            app.step = AppStep::Apply;
                        }
                    }
                    AppStep::Apply => {
                        if key.code == KeyCode::Enter {
                            app.error_message.clear();
                            match do_apply(&mut app) {
                                Ok(count) => {
                                    app.status_message =
                                        tf("apply_done", &[("count", &count.to_string())]);
                                    let branch = git_ops::get_current_branch(Path::new(
                                        &app.config.mine_path,
                                    ))
                                    .unwrap_or_else(|_| "main".to_string());
                                    let origin = git_ops::get_origin_url(Path::new(&app.config.mine_path));
                                    app.git_push_state = Some(GitPushState::new(&branch, &origin));
                                    app.step = AppStep::GitPush;
                                }
                                Err(e) => {
                                    app.error_message =
                                        tf("apply_error", &[("err", &e.to_string())]);
                                }
                            }
                        }
                    }
                    AppStep::GitPush => {
                        let is_done = app.git_push_state.as_ref().map(|gp| gp.done).unwrap_or(false);
                        let is_pushing = app.git_push_state.as_ref().map(|gp| gp.pushing).unwrap_or(false);
                        if is_done {
                            // Push finished, any key goes to Done
                            app.step = AppStep::Done;
                        } else if !is_pushing {
                            let should_proceed = app
                                .git_push_state
                                .as_mut()
                                .map(|gp| gp.handle_key(key))
                                .unwrap_or(false);
                            if should_proceed {
                                start_push(&mut app);
                            }
                        }
                    }
                    AppStep::Done => {
                        if key.code == KeyCode::Enter {
                            app.reset_for_new_task();
                        } else if key.code == KeyCode::Char('q') {
                            break;
                        }
                    }
                }
            }
            _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
