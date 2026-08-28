use std::env;
use std::fs::{create_dir_all, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{mpsc, RwLock as TokioRwLock};

mod config;
mod dungeon;
mod errors;
mod history;
mod lb;
mod model;
mod parse;
mod theme;
mod ui;
mod ws_client;

use history::{
    determine_history_task, handle_history_mouse, handle_history_settings_input,
    open_history_settings_panel, refresh_archive_count, shared_retention_state,
    spawn_history_delete, spawn_history_task, spawn_initial_history_loads,
    try_close_history_settings, HistoryRetentionPolicy, HistorySession, HistorySessionHandle,
    HistorySettingsContext, HistoryStore,
};
use model::{AppEvent, AppSettings, AppState, SettingsField, WS_URL_DEFAULT};
use tracing::level_filters::LevelFilter;
use tracing::warn;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_cli()?;
    init_tracing(&cli)?;

    let state = Arc::new(TokioRwLock::new(AppState::default()));

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let event_tx = tx.clone();

    let dungeon_catalog = match dungeon::DungeonCatalog::load_default() {
        Ok(catalog) => Some(Arc::new(catalog)),
        Err(err) => {
            warn!(error = ?err, "Dungeon catalog unavailable; dungeon mode disabled");
            None
        }
    };

    let app_cfg = match config::load() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to load config: {err:?}. Using defaults.");
            config::AppConfig::default()
        }
    };

    let retention_state = shared_retention_state(&app_cfg);

    {
        let mut s = state.write().await;
        s.apply_settings(AppSettings::from(app_cfg.clone()));
        if s.disconnected_since.is_none() {
            s.disconnected_since = Some(Instant::now());
        }
        refresh_archive_count(&mut s);
    }

    let history_session = HistorySessionHandle::new(HistorySession::new(
        tx.clone(),
        dungeon_catalog.clone(),
        app_cfg.dungeon_mode_enabled,
        Arc::clone(&retention_state),
    ));

    if app_cfg.history_enabled {
        history_session.enable().await?;
        if HistoryRetentionPolicy::from_config(&app_cfg).is_applied_in_config(&app_cfg) {
            let policy = HistoryRetentionPolicy::from_config(&app_cfg);
            let _ = history_session
                .apply_retention_if_applied(&policy, &app_cfg)
                .await;
        }
    }

    let mut hs_ctx = HistorySettingsContext {
        session: history_session.clone(),
        retention_state: Arc::clone(&retention_state),
        app_cfg,
        event_tx: event_tx.clone(),
        view_store: None,
    };

    let ws_url = WS_URL_DEFAULT.to_string();
    let ws_history = history_session.clone();
    let ws_tx = tx.clone();
    tokio::spawn(async move { ws_client::run(ws_url, ws_tx, ws_history).await });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick = Duration::from_millis(100);
    let mut last_draw = Instant::now();
    let mut running = true;

    while running {
        while let Ok(evt) = rx.try_recv() {
            let mut s = state.write().await;
            s.apply(evt);
        }

        if last_draw.elapsed() >= tick {
            let mut s = state.write().await;
            let snapshot = s.clone_snapshot();
            let mut list_state = std::mem::take(&mut s.history_list_state);
            let mut offset = 0usize;
            let draw_result = terminal.draw(|f| {
                offset = ui::draw(f, &snapshot, &mut list_state);
            });
            s.history_list_state = list_state;
            s.history.list_scroll_offset = offset;
            draw_result?;
            last_draw = Instant::now();
        }

        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let mut s = state.write().await;

                    if s.history_settings.visible
                        && handle_history_settings_input(key, &mut s, &mut hs_ctx).await
                    {
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if s.history.visible && s.history.confirm.is_some() {
                                s.history_cancel_confirm();
                                continue;
                            }
                            if try_close_history_settings(&mut s).await {
                                continue;
                            }
                            if s.show_settings {
                                s.show_settings = false;
                            } else if s.history.visible {
                                if s.history.viewing_archive.is_some()
                                    && s.history.level == model::HistoryPanelLevel::Dates
                                    && s.history.dungeon_level == model::DungeonPanelLevel::Dates
                                {
                                    s.history.visible = false;
                                    s.history.reset();
                                    hs_ctx.view_store = None;
                                } else if s.history.viewing_archive.is_some() {
                                    s.history.back();
                                } else {
                                    s.history.visible = false;
                                    s.history.reset();
                                }
                            } else {
                                running = false;
                            }
                        }
                        KeyCode::Char('h') => {
                            if s.history.viewing_archive.is_some() {
                                if s.history.level == model::HistoryPanelLevel::Dates
                                    && s.history.dungeon_level == model::DungeonPanelLevel::Dates
                                {
                                    s.history.visible = false;
                                    s.history.reset();
                                    hs_ctx.view_store = None;
                                } else {
                                    s.history.back();
                                }
                                continue;
                            }
                            let should_load = s.toggle_history();
                            if should_load {
                                if let Some(store) = active_store(&hs_ctx, &history_session).await {
                                    spawn_initial_history_loads(
                                        &mut s.history,
                                        store,
                                        event_tx.clone(),
                                    );
                                }
                            }
                        }
                        KeyCode::Char('i') => {
                            if !s.history.visible {
                                let now = Instant::now();
                                if s.is_idle_at(now) {
                                    s.show_idle_overlay = !s.show_idle_overlay;
                                }
                            }
                        }
                        _ => {
                            let mut pending_task = None;
                            let mut pending_delete = None;
                            let history_active = if s.history.visible {
                                if s.history.confirm.is_some() {
                                    match key.code {
                                        KeyCode::Up => s.history_confirm_cycle(-1),
                                        KeyCode::Down => s.history_confirm_cycle(1),
                                        KeyCode::Esc => s.history_cancel_confirm(),
                                        KeyCode::Enter => {
                                            let is_cancel = s
                                                .history
                                                .confirm
                                                .as_ref()
                                                .is_some_and(|confirm| confirm.focus == 0);
                                            if is_cancel {
                                                s.history_cancel_confirm();
                                            } else if let Some(action) =
                                                s.history_take_confirm_action()
                                            {
                                                s.history_begin_load();
                                                pending_delete = Some(action);
                                            }
                                        }
                                        _ => {}
                                    }
                                } else {
                                    match key.code {
                                        KeyCode::Up => s.history_move_selection(-1),
                                        KeyCode::Down => s.history_move_selection(1),
                                        KeyCode::PageUp => s.history_move_selection(-5),
                                        KeyCode::PageDown => s.history_move_selection(5),
                                        KeyCode::Left | KeyCode::Backspace => s.history_back(),
                                        KeyCode::Right | KeyCode::Enter => s.history_enter(),
                                        KeyCode::Char('m') | KeyCode::Char('M') => {
                                            s.history_toggle_mode()
                                        }
                                        KeyCode::Tab => s.history_toggle_view(),
                                        KeyCode::Char('t') | KeyCode::Char('T') => {
                                            s.history_toggle_view()
                                        }
                                        KeyCode::Char('D')
                                            if key.modifiers.contains(KeyModifiers::SHIFT) =>
                                        {
                                            s.history_begin_delete();
                                        }
                                        _ => {}
                                    }
                                    pending_task = determine_history_task(&mut s);
                                }
                                true
                            } else {
                                false
                            };

                            if let Some(action) = pending_delete {
                                if let Some(store) = active_store(&hs_ctx, &history_session).await {
                                    spawn_history_delete(store, event_tx.clone(), action);
                                } else {
                                    s.history.finish_load();
                                    s.history.error = Some("History store unavailable".to_string());
                                }
                            }

                            if let Some(task) = pending_task {
                                if let Some(store) = active_store(&hs_ctx, &history_session).await {
                                    spawn_history_task(task, store, event_tx.clone());
                                }
                            }

                            if history_active {
                                continue;
                            }

                            match key.code {
                                KeyCode::Char('D')
                                    if key.modifiers.contains(KeyModifiers::SHIFT) =>
                                {
                                    history_session.cut_dungeon_session().await;
                                }
                                KeyCode::Char('d') => {
                                    s.decoration = s.decoration.next();
                                }
                                KeyCode::Char('m') => {
                                    s.mode = s.mode.next();
                                    s.resort_rows();
                                }
                                KeyCode::Char('s') => {
                                    s.show_settings = !s.show_settings;
                                    if s.show_settings {
                                        s.settings_cursor = SettingsField::default();
                                    }
                                }
                                KeyCode::Enter if s.show_settings => {
                                    if s.settings_cursor == SettingsField::HistorySettings {
                                        let live_size = history_session.live_db_size_bytes().await;
                                        open_history_settings_panel(&mut s, live_size);
                                    }
                                }
                                KeyCode::Up if s.show_settings => {
                                    s.prev_setting();
                                }
                                KeyCode::Down if s.show_settings => {
                                    s.next_setting();
                                }
                                KeyCode::Left | KeyCode::Right if s.show_settings => {
                                    let forward = matches!(key.code, KeyCode::Right);
                                    if s.adjust_selected_setting(forward) {
                                        hs_ctx.app_cfg =
                                            config::AppConfig::from(s.settings.clone());
                                        if let Err(err) = config::save(&hs_ctx.app_cfg) {
                                            eprintln!("Failed to save config: {err:?}");
                                        }
                                        history_session
                                            .set_dungeon_mode_enabled(
                                                hs_ctx.app_cfg.dungeon_mode_enabled,
                                            )
                                            .await;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Event::Key(_) => {}
                Event::Mouse(mouse) => {
                    let list_offset = {
                        let s = state.read().await;
                        s.history.list_scroll_offset
                    };
                    handle_history_mouse(mouse, &state, list_offset).await;
                    let mut s = state.write().await;
                    if s.history.visible {
                        if let Some(task) = determine_history_task(&mut s) {
                            if let Some(store) = active_store(&hs_ctx, &history_session).await {
                                spawn_history_task(task, store, event_tx.clone());
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
    history_session.shutdown().await;
    Ok(())
}

async fn active_store(
    ctx: &HistorySettingsContext,
    session: &HistorySessionHandle,
) -> Option<Arc<HistoryStore>> {
    if let Some(view) = ctx.view_store.clone() {
        return Some(view);
    }
    session.store().await
}

#[derive(Debug, Default)]
struct CliArgs {
    debug: Option<DebugTarget>,
}

#[derive(Debug)]
enum DebugTarget {
    Default,
    Path(PathBuf),
}

fn parse_cli() -> Result<CliArgs> {
    let mut args = env::args().skip(1).peekable();
    let mut debug = None;

    while let Some(arg) = args.next() {
        if arg == "--debug" {
            if debug.is_some() {
                bail!("`--debug` specified more than once");
            }
            if let Some(next) = args.peek() {
                if !next.starts_with('-') {
                    let path = args
                        .next()
                        .map(PathBuf::from)
                        .expect("peek ensured next exists");
                    debug = Some(DebugTarget::Path(path));
                    continue;
                }
            }
            debug = Some(DebugTarget::Default);
        } else if let Some(rest) = arg.strip_prefix("--debug=") {
            if debug.is_some() {
                bail!("`--debug` specified more than once");
            }
            if rest.is_empty() {
                debug = Some(DebugTarget::Default);
            } else {
                debug = Some(DebugTarget::Path(PathBuf::from(rest)));
            }
        } else {
            bail!("unknown argument: {arg}");
        }
    }

    Ok(CliArgs { debug })
}

fn init_tracing(cli: &CliArgs) -> Result<()> {
    if let Some(target) = &cli.debug {
        let log_path = match target {
            DebugTarget::Default => config::config_dir().join("debug.log"),
            DebugTarget::Path(path) => path.clone(),
        };

        if let Some(parent) = log_path.parent() {
            if !parent.as_os_str().is_empty() {
                create_dir_all(parent).with_context(|| {
                    format!("failed to create log directory {}", parent.display())
                })?;
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("failed to open log file {}", log_path.display()))?;

        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || file.try_clone().expect("failed to clone log file handle"))
            .with_ansi(false)
            .with_target(false)
            .with_max_level(LevelFilter::DEBUG);

        subscriber.try_init().map_err(|err| {
            anyhow::anyhow!(
                "failed to initialize logging to {}: {}",
                log_path.display(),
                err
            )
        })?;
    }

    Ok(())
}
