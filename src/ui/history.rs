use chrono::{Local, TimeZone};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::history::util::{format_duration_label, format_number};
use crate::model::{AppSnapshot, DungeonPanelLevel, HistoryPanelLevel, HistoryView, ViewMode};
use crate::theme::{accent_color, header_style, text_color, title_style, value_style};
use crate::ui::{draw_encounter_record, EncounterDetailParams};

pub fn draw_history(f: &mut Frame, s: &AppSnapshot, list_state: &mut ListState) -> usize {
    let area = f.size();
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .margin(0)
        .split(area);

    draw_header(f, chunks[0], s);
    draw_body(f, chunks[1], s, list_state);
    if let Some(confirm) = &s.history.confirm {
        draw_history_confirm(f, confirm);
    }
    list_state.offset()
}

fn draw_header(f: &mut Frame, area: Rect, s: &AppSnapshot) {
    let subtitle = if s.history.loading {
        "Loading history…"
    } else if let Some(err) = &s.history.error {
        err.as_str()
    } else {
        match (s.history.view, s.history.level, s.history.dungeon_level) {
            (HistoryView::Encounters, HistoryPanelLevel::Dates, _) => {
                "Enter/Click ▸ view encounters · Shift+D delete · ↑/↓ scroll · Tab switches view"
            }
            (HistoryView::Encounters, HistoryPanelLevel::Encounters, _) => {
                "← dates · Shift+D delete · ↑/↓ scroll · Enter view details · Tab switches view"
            }
            (HistoryView::Encounters, HistoryPanelLevel::EncounterDetail, _) => {
                "← encounters · Shift+D delete · ↑/↓ switch encounter · m toggles DPS/Heal · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::Dates) => {
                "Enter/Click ▸ view runs · Shift+D delete · ↑/↓ scroll · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::Runs) => {
                "← dates · Shift+D delete · ↑/↓ scroll · Enter view run · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::RunDetail) => {
                "← runs · Shift+D delete · ↑/↓ select pull · Enter view pull · m toggles table · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::EncounterDetail) => {
                "← run detail · ↑/↓ switch pull · m toggles DPS/Heal · Tab switches view"
            }
        }
    };

    let (enc_style, dun_style) = if s.history.view == HistoryView::Encounters {
        (title_style().add_modifier(Modifier::BOLD), header_style())
    } else {
        (header_style(), title_style().add_modifier(Modifier::BOLD))
    };

    let tabs_line = Line::from(vec![
        Span::styled("Encounters", enc_style),
        Span::raw("  |  "),
        Span::styled("Dungeons", dun_style),
    ]);

    let title_line = Line::from(vec![Span::styled(
        "History",
        Style::default()
            .fg(accent_color())
            .add_modifier(Modifier::BOLD),
    )]);
    let subtitle_line = Line::from(vec![Span::styled(
        subtitle,
        Style::default().fg(text_color()),
    )]);

    let title = if let Some(name) = &s.history.viewing_archive {
        format!("History · archive: {name}")
    } else {
        "History".to_string()
    };

    let block = Paragraph::new(vec![title_line, tabs_line, subtitle_line])
        .alignment(ratatui::layout::Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(block, area);
}

fn draw_body(f: &mut Frame, area: Rect, s: &AppSnapshot, list_state: &mut ListState) {
    if let Some(err) = &s.history.error {
        let block = Paragraph::new(err.as_str())
            .alignment(ratatui::layout::Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Error"));
        f.render_widget(block, area);
        return;
    }

    let is_loading = s.history.loading;

    match s.history.view {
        HistoryView::Encounters => {
            if s.history.days.is_empty() {
                let message = if is_loading {
                    "Loading history…"
                } else {
                    "No encounters recorded yet."
                };
                let block = Paragraph::new(message)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(block, area);
                if is_loading {
                    render_loading_overlay(f, area, "Loading…");
                }
                return;
            }
            match s.history.level {
                HistoryPanelLevel::Dates => draw_dates(f, area, s, list_state),
                HistoryPanelLevel::Encounters => draw_encounters(f, area, s, list_state),
                HistoryPanelLevel::EncounterDetail => draw_encounter_detail(f, area, s),
            }
        }
        HistoryView::Dungeons => {
            if s.history.dungeon_days.is_empty() {
                let message = if is_loading {
                    "Loading dungeon history…"
                } else {
                    "No dungeon runs recorded yet."
                };
                let block = Paragraph::new(message)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(block, area);
                if is_loading {
                    render_loading_overlay(f, area, "Loading…");
                }
                return;
            }
            match s.history.dungeon_level {
                DungeonPanelLevel::Dates => draw_dungeon_dates(f, area, s, list_state),
                DungeonPanelLevel::Runs => draw_dungeon_runs(f, area, s, list_state),
                DungeonPanelLevel::RunDetail => draw_dungeon_run_detail(f, area, s, list_state),
                DungeonPanelLevel::EncounterDetail => draw_dungeon_encounter_detail(f, area, s),
            }
        }
    }

    if is_loading {
        render_loading_overlay(f, area, "Loading…");
    }
}

fn draw_dates(f: &mut Frame, area: Rect, s: &AppSnapshot, list_state: &mut ListState) {
    if s.history.days.is_empty() {
        let block = Paragraph::new("No encounters recorded yet.")
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    }

    let items: Vec<ListItem> = s
        .history
        .days
        .iter()
        .map(|day| ListItem::new(day.label.clone()))
        .collect();

    list_state.select(Some(s.history.selected_day));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Dates"))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(accent_color())
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[0], list_state);

    let hint = Paragraph::new("Tab swaps view · Enter view encounters · Shift+D delete")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, chunks[1]);
}

fn draw_encounters(f: &mut Frame, area: Rect, s: &AppSnapshot, list_state: &mut ListState) {
    let Some(day) = s.history.current_day() else {
        let block = Paragraph::new("No date selected.")
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    if !day.encounters_loaded && !day.encounter_ids.is_empty() {
        let block = Paragraph::new("Loading encounters…")
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    }

    if day.encounters.is_empty() {
        let block = Paragraph::new("No encounters captured for this date.")
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    }

    let items: Vec<ListItem> = day
        .encounters
        .iter()
        .map(|enc| {
            let text = format!("{}  [{}]", enc.display_title, enc.time_label);
            ListItem::new(text)
        })
        .collect();

    list_state.select(Some(s.history.selected_encounter));

    let title = format!("Encounters · {}", day.label);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(accent_color())
                .add_modifier(Modifier::BOLD),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    f.render_stateful_widget(list, chunks[0], list_state);

    let hint = Paragraph::new("Enter view details · Shift+D delete")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, chunks[1]);
}

fn draw_encounter_detail(f: &mut Frame, area: Rect, s: &AppSnapshot) {
    let Some(day) = s.history.current_day() else {
        let block = Paragraph::new("No date selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let Some(encounter) = day.encounters.get(s.history.selected_encounter) else {
        let block = Paragraph::new("No encounter selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let Some(record) = encounter.record.as_deref() else {
        let block = Paragraph::new("Loading encounter…")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![Span::styled(
                        format!("Details · {}", encounter.display_title),
                        title_style(),
                    )])),
            );
        f.render_widget(block, area);
        return;
    };

    draw_encounter_record(
        f,
        area,
        EncounterDetailParams {
            record,
            title: encounter.display_title.clone(),
            zone_fallback: "Unknown".to_string(),
            last_seen_label: encounter.timestamp_label.clone(),
            detail_mode: s.history.detail_mode,
            decoration: s.decoration,
            limit_break_mode: s.settings.limit_break_mode,
            footer_hint: "← back · ↑/↓ switch encounter · Shift+D delete · m toggles DPS/Heal",
        },
    );
}

fn draw_dungeon_dates(f: &mut Frame, area: Rect, s: &AppSnapshot, list_state: &mut ListState) {
    let items: Vec<ListItem> = s
        .history
        .dungeon_days
        .iter()
        .map(|day| ListItem::new(day.label.clone()))
        .collect();

    list_state.select(Some(s.history.dungeon_selected_day));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Dungeon Dates"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(accent_color())
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[0], list_state);

    let hint = Paragraph::new("Tab swaps view · Enter view runs · Shift+D delete")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, chunks[1]);
}

fn draw_dungeon_runs(f: &mut Frame, area: Rect, s: &AppSnapshot, list_state: &mut ListState) {
    let Some(day) = s.history.current_dungeon_day() else {
        let block = Paragraph::new("No date selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    if !day.runs_loaded && !day.run_ids.is_empty() {
        let block = Paragraph::new("Loading runs…")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    }

    if day.runs.is_empty() {
        let block = Paragraph::new("No dungeon runs captured for this date.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    }

    let items: Vec<ListItem> = day
        .runs
        .iter()
        .map(|run| {
            let mut text = format!(
                "{} · {} · pulls: {} · dmg {} · dps {}",
                run.zone,
                run.started_label,
                run.child_count,
                format_number(run.total_damage),
                format_number(run.total_encdps),
            );
            if run.incomplete {
                text.push_str(" · incomplete");
            }
            ListItem::new(text)
        })
        .collect();

    list_state.select(Some(s.history.dungeon_selected_run));

    let title = format!("Dungeon Runs · {}", day.label);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(accent_color())
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, list_state);
}

fn draw_dungeon_run_detail(f: &mut Frame, area: Rect, s: &AppSnapshot, list_state: &mut ListState) {
    let Some(day) = s.history.current_dungeon_day() else {
        let block = Paragraph::new("No date selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let Some(run) = day.runs.get(s.history.dungeon_selected_run) else {
        let block = Paragraph::new("No run selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let Some(record) = run.record.as_ref() else {
        let block = Paragraph::new("Loading run…")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let party = if record.party_signature.is_empty() {
        "Unknown".to_string()
    } else {
        format_party_signature(&record.party_signature)
    };

    let detail_mode = s.history.dungeon_detail_mode;
    let (total_label, total_value, average_label, average_value) = match detail_mode {
        ViewMode::Dps => (
            "Total Damage",
            format_number(record.total_damage),
            "Average DPS",
            format_number(record.total_encdps),
        ),
        ViewMode::Heal => {
            let avg_hps = if record.total_duration_secs > 0 {
                record.total_healed / record.total_duration_secs as f64
            } else {
                0.0
            };
            (
                "Total Healed",
                format_number(record.total_healed),
                "Average HPS",
                format_number(avg_hps),
            )
        }
    };

    let mut summary_lines = Vec::new();
    summary_lines.push(Line::from(vec![
        Span::styled("Zone: ", header_style()),
        Span::styled(record.zone.clone(), value_style()),
    ]));
    summary_lines.push(Line::from(vec![
        Span::styled("Duration: ", header_style()),
        Span::styled(
            format_duration_label(record.total_duration_secs),
            value_style(),
        ),
    ]));
    summary_lines.push(Line::from(vec![
        Span::styled(format!("{total_label}: "), header_style()),
        Span::styled(total_value, value_style()),
        Span::raw(" · "),
        Span::styled(format!("{average_label}: "), header_style()),
        Span::styled(average_value, value_style()),
    ]));
    if matches!(detail_mode, ViewMode::Dps) {
        summary_lines.push(Line::from(vec![
            Span::styled("Total Healed: ", header_style()),
            Span::styled(format_number(record.total_healed), value_style()),
        ]));
    } else {
        summary_lines.push(Line::from(vec![
            Span::styled("Total Damage: ", header_style()),
            Span::styled(format_number(record.total_damage), value_style()),
        ]));
    }
    summary_lines.push(Line::from(vec![
        Span::styled("Party: ", header_style()),
        Span::styled(party, value_style()),
    ]));
    if record.incomplete {
        summary_lines.push(Line::from(vec![Span::styled(
            "Status: Incomplete",
            title_style().add_modifier(Modifier::BOLD),
        )]));
    }

    let mut list_items = Vec::new();
    let metric_label = match detail_mode {
        ViewMode::Dps => "DPS",
        ViewMode::Heal => "HPS",
    };

    for (idx, title) in record.child_titles.iter().enumerate() {
        let label = if let Some(child) = run.child_records.get(idx).and_then(|c| c.as_deref()) {
            let metric_value = match detail_mode {
                ViewMode::Dps => child.encounter.encdps.as_str(),
                ViewMode::Heal => child.encounter.enchps.as_str(),
            };
            let metric_value = if metric_value.is_empty() {
                "—"
            } else {
                metric_value
            };
            format!(
                "{} · {} · {} {}",
                title, child.encounter.duration, metric_label, metric_value,
            )
        } else {
            format!("{} · (loading…)", title)
        };
        list_items.push(ListItem::new(label));
    }

    if !list_items.is_empty() {
        list_state.select(Some(s.history.dungeon_selected_child));
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_lines.len().saturating_add(2) as u16),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(area);

    let summary = Paragraph::new(summary_lines)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![Span::styled(
                    format!("Run · {}", run.zone),
                    title_style(),
                )])),
        );
    f.render_widget(summary, layout[0]);

    if list_items.is_empty() {
        let block = Paragraph::new("No pulls recorded in this run.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, layout[1]);
    } else {
        let title = format!("Pulls · {}", record.child_keys.len());
        let list = List::new(list_items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_stateful_widget(list, layout[1], list_state);
    }

    let instructions = Paragraph::new(
        "← runs · Shift+D delete · ↑/↓ select pull · Enter view pull · m toggles DPS/Heal",
    )
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::NONE));
    f.render_widget(instructions, layout[2]);
}

fn draw_dungeon_encounter_detail(f: &mut Frame, area: Rect, s: &AppSnapshot) {
    let Some(run) = s.history.current_dungeon_run() else {
        let block = Paragraph::new("No run selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let Some(parent_record) = run.record.as_ref() else {
        let block = Paragraph::new("Loading run…")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let idx = s.history.dungeon_selected_child;
    if idx >= parent_record.child_keys.len() {
        let block = Paragraph::new("No pull selected.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    }

    let Some(encounter_record) = run.child_records.get(idx).and_then(|c| c.as_deref()) else {
        let block = Paragraph::new("Loading encounter…")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, area);
        return;
    };

    let title = parent_record
        .child_titles
        .get(idx)
        .cloned()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let t = encounter_record.encounter.title.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        })
        .unwrap_or_else(|| "Encounter".to_string());

    draw_encounter_record(
        f,
        area,
        EncounterDetailParams {
            record: encounter_record,
            title,
            zone_fallback: run.zone.clone(),
            last_seen_label: format_timestamp_label(encounter_record.last_seen_ms),
            detail_mode: s.history.detail_mode,
            decoration: s.decoration,
            limit_break_mode: s.settings.limit_break_mode,
            footer_hint: "← run detail · ↑/↓ switch pull · m toggles DPS/Heal · Enter re-open",
        },
    );
}

fn render_loading_overlay(f: &mut Frame, area: Rect, message: &str) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text_width = message.chars().count() as u16 + 4;
    let overlay_width = text_width.min(area.width);
    let overlay_height = 3.min(area.height).max(1);
    let x = area.x + (area.width.saturating_sub(overlay_width)) / 2;
    let y = area.y + (area.height.saturating_sub(overlay_height)) / 2;
    let overlay = Rect {
        x,
        y,
        width: overlay_width,
        height: overlay_height,
    };
    f.render_widget(Clear, overlay);
    let block = Paragraph::new(message)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(block, overlay);
}

fn format_timestamp_label(ms: u64) -> String {
    if let Ok(ms_i64) = i64::try_from(ms) {
        if let Some(dt) = Local.timestamp_millis_opt(ms_i64).single() {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }
    "unknown".to_string()
}

fn format_party_signature(sig: &[String]) -> String {
    if sig.is_empty() {
        return "Unknown".to_string();
    }
    sig.join(", ")
}

fn draw_history_confirm(f: &mut Frame, confirm: &crate::model::HistoryConfirm) {
    let area = centered_rect(70, 45, f.size());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::default()];
    for paragraph in confirm.message.lines() {
        lines.push(Line::from(vec![Span::raw(paragraph.to_string())]));
    }
    lines.push(Line::default());

    for (idx, (label, _)) in confirm.options.iter().enumerate() {
        let style = if idx == confirm.focus {
            title_style()
        } else {
            header_style()
        };
        let marker = if idx == confirm.focus { "▶" } else { " " };
        lines.push(Line::from(vec![Span::styled(
            format!("{marker} {label}"),
            style,
        )]));
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "↑/↓ choose · Enter confirm · Esc cancel",
        header_style(),
    )]));

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "Confirm Delete",
            title_style(),
        )]))
        .borders(Borders::ALL);
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
