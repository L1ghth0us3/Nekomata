use std::env;
use std::fs::{create_dir_all, OpenOptions};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{io, sync::Arc};

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
use tokio::sync::{mpsc, RwLock};

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
    determine_history_task, handle_history_mouse, spawn_history_task, spawn_initial_history_loads,
    HistoryStore,
};
use model::{
    AppEvent, AppSettings, AppState, SettingsField, WS_URL_DEFAULT,
};
use tracing::level_filters::LevelFilter;
use tracing::warn;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_cli()?;
    init_tracing(&cli)?;

    // Shared app state
    let state = Arc::new(RwLock::new(AppState::default()));

    // WS event channel
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let event_tx = tx.clone();

    // Dungeon catalog (optional; disable dungeon mode if unavailable)
    let dungeon_catalog = match dungeon::DungeonCatalog::load_default() {
        Ok(catalog) => Some(Arc::new(catalog)),
        Err(err) => {
            warn!(error = ?err, "Dungeon catalog unavailable; dungeon mode disabled");
            None
        }
    };

    // Load persisted configuration into state
    let app_cfg = match config::load() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to load config: {err:?}. Using defaults.");
            config::AppConfig::default()
        }
    };
    {
        let mut s = state.write().await;
        s.apply_settings(AppSettings::from(app_cfg.clone()));
        // Initialize disconnected_since since the app starts disconnected
        // This must happen after settings are loaded so idle_duration() works correctly
        if s.disconnected_since.is_none() {
            s.disconnected_since = Some(Instant::now());
        }
    }

    // History persistence (sled-backed)
    let history_store = Arc::new(history::HistoryStore::open_default()?);
    let history_recorder = history::spawn_recorder(
        history_store.clone(),
        tx.clone(),
        dungeon_catalog.clone(),
        app_cfg.dungeon_mode_enabled,
    );

    // Spawn WS client task (auto-connect and subscribe)
    let ws_url = WS_URL_DEFAULT.to_string();
    let history_tx = history_recorder.clone();
    let ws_tx = tx.clone();
    tokio::spawn(async move { ws_client::run(ws_url, ws_tx, history_tx).await });

    // TUI init
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App loop
    let tick = Duration::from_millis(100);
    let mut last_draw = Instant::now();
    let mut running = true;

    while running {
        // Drain any incoming WS events into state
        while let Ok(evt) = rx.try_recv() {
            let mut s = state.write().await;
            s.apply(evt);
        }

        // Draw at most every tick interval or immediately on first loop
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

        // Non-blocking input with small timeout so we keep redrawing
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        let mut s = state.write().await;
                        if s.show_settings {
                            s.show_settings = false;
                        } else if s.history.visible {
                            s.history.visible = false;
                            s.history.reset();
                        } else {
                            running = false;
                        }
                    }
                    KeyCode::Char('h') => {
                        let should_load = {
                            let mut s = state.write().await;
                            s.toggle_history()
                        };
                        if should_load {
                            let mut s = state.write().await;
                            spawn_initial_history_loads(
                                &mut s.history,
                                history_store.clone(),
                                event_tx.clone(),
                            );
                        }
                    }
                    KeyCode::Char('i') => {
                        let mut s = state.write().await;
                        if !s.history.visible {
                            let now = Instant::now();
                            if s.is_idle_at(now) {
                                s.show_idle_overlay = !s.show_idle_overlay;
                            }
                        }
                    }
                    _ => {
                        let mut pending_task = None;
                        let history_active = {
                            let mut s = state.write().await;
                            if s.history.visible {
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
                                    _ => {}
                                }
                                pending_task = determine_history_task(&mut s);
                                true
                            } else {
                                false
                            }
                        };

                        if let Some(task) = pending_task {
                            spawn_history_task(task, history_store.clone(), event_tx.clone());
                        }

                        if history_active {
                            continue;
                        }

                        match key.code {
                            KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                history_recorder.cut_dungeon_session();
                            }
                            KeyCode::Char('d') => {
                                let mut s = state.write().await;
                                s.decoration = s.decoration.next();
                            }
                            KeyCode::Char('m') => {
                                let mut s = state.write().await;
                                s.mode = s.mode.next();
                                s.resort_rows();
                            }
                            KeyCode::Char('s') => {
                                let mut s = state.write().await;
                                s.show_settings = !s.show_settings;
                                if s.show_settings {
                                    s.settings_cursor = SettingsField::default();
                                }
                            }
                            KeyCode::Up => {
                                let mut s = state.write().await;
                                if s.show_settings {
                                    s.prev_setting();
                                }
                            }
                            KeyCode::Down => {
                                let mut s = state.write().await;
                                if s.show_settings {
                                    s.next_setting();
                                }
                            }
                            KeyCode::Left | KeyCode::Right => {
                                let forward = matches!(key.code, KeyCode::Right);
                                let updated = {
                                    let mut s = state.write().await;
                                    if s.show_settings && s.adjust_selected_setting(forward) {
                                        Some(s.settings.clone())
                                    } else {
                                        None
                                    }
                                };
                                if let Some(settings) = updated {
                                    let app_cfg: config::AppConfig = settings.into();
                                    if let Err(err) = config::save(&app_cfg) {
                                        eprintln!("Failed to save config: {err:?}");
                                    }
                                    history_recorder
                                        .set_dungeon_mode_enabled(app_cfg.dungeon_mode_enabled);
                                }
                            }
                            _ => {}
                        }
                    }
                },
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
                            spawn_history_task(task, history_store.clone(), event_tx.clone());
                        }
                    }
                }
                _ => {}
            }
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
    history_recorder.shutdown().await;
    Ok(())
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

