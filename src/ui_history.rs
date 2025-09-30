use std::cmp::Ordering;

use chrono::{Local, TimeZone};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::model::{
    AppSnapshot, CombatantRow, DungeonPanelLevel, HistoryPanelLevel, HistoryView, ViewMode,
};
use crate::theme::{header_style, title_style, value_style, TEXT};
use crate::ui::{draw_table_with_context, TableRenderContext};

pub fn draw_history(f: &mut Frame, s: &AppSnapshot) {
    let area = f.size();
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .margin(0)
        .split(area);

    draw_header(f, chunks[0], s);
    draw_body(f, chunks[1], s);
}

fn draw_header(f: &mut Frame, area: Rect, s: &AppSnapshot) {
    let subtitle = if s.history.loading {
        "Loading history…"
    } else if let Some(err) = &s.history.error {
        err.as_str()
    } else {
        match (s.history.view, s.history.level, s.history.dungeon_level) {
            (HistoryView::Encounters, HistoryPanelLevel::Dates, _) => {
                "Enter/Click ▸ view encounters · ↑/↓ scroll · Tab switches view"
            }
            (HistoryView::Encounters, HistoryPanelLevel::Encounters, _) => {
                "← dates · ↑/↓ scroll · Enter view details · Tab switches view"
            }
            (HistoryView::Encounters, HistoryPanelLevel::EncounterDetail, _) => {
                "← encounters · ↑/↓ switch encounter · m toggles DPS/Heal · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::Dates) => {
                "Enter/Click ▸ view runs · ↑/↓ scroll · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::Runs) => {
                "← dates · ↑/↓ scroll · Enter view run · Tab switches view"
            }
            (HistoryView::Dungeons, _, DungeonPanelLevel::RunDetail) => {
                "← runs · ↑/↓ select pull · Enter view pull · m toggles table · Tab switches view"
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
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]);
    let subtitle_line = Line::from(vec![Span::styled(subtitle, Style::default().fg(TEXT))]);

    let block = Paragraph::new(vec![title_line, tabs_line, subtitle_line])
        .alignment(ratatui::layout::Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title("History"));
    f.render_widget(block, area);
}

fn draw_body(f: &mut Frame, area: Rect, s: &AppSnapshot) {
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
                HistoryPanelLevel::Dates => draw_dates(f, area, s),
                HistoryPanelLevel::Encounters => draw_encounters(f, area, s),
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
                DungeonPanelLevel::Dates => draw_dungeon_dates(f, area, s),
                DungeonPanelLevel::Runs => draw_dungeon_runs(f, area, s),
                DungeonPanelLevel::RunDetail => draw_dungeon_run_detail(f, area, s),
                DungeonPanelLevel::EncounterDetail => draw_dungeon_encounter_detail(f, area, s),
            }
        }
    }

    if is_loading {
        render_loading_overlay(f, area, "Loading…");
    }
}

fn draw_dates(f: &mut Frame, area: Rect, s: &AppSnapshot) {
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

    let mut state = ListState::default();
    state.select(Some(s.history.selected_day));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Dates"))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[0], &mut state);

    let hint = Paragraph::new("Tab swaps view · Enter view encounters")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, chunks[1]);
}

fn draw_encounters(f: &mut Frame, area: Rect, s: &AppSnapshot) {
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

    let mut state = ListState::default();
    state.select(Some(s.history.selected_encounter));

    let title = format!("Encounters · {}", day.label);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
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

    let Some(record) = encounter.record.as_ref() else {
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

    let basic_metrics = [
        (
            "Encounter",
            if record.encounter.title.is_empty() {
                encounter.display_title.clone()
            } else {
                record.encounter.title.clone()
            },
        ),
        (
            "Zone",
            if record.encounter.zone.is_empty() {
                "Unknown".to_string()
            } else {
                record.encounter.zone.clone()
            },
        ),
        ("Duration", record.encounter.duration.clone()),
        ("ENCDPS", record.encounter.encdps.clone()),
        ("Damage", record.encounter.damage.clone()),
    ];

    let technical_metrics = [
        ("Snapshots", record.snapshots.to_string()),
        ("Frames", record.frames.len().to_string()),
        ("Last seen", encounter.timestamp_label.clone()),
    ];

    let summary_lines: Vec<Line> = basic_metrics
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(format!("{label}: "), header_style()),
                Span::styled(value.clone(), value_style()),
            ])
        })
        .collect();

    let technical_lines: Vec<Line> = technical_metrics
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(format!("{label}: "), header_style()),
                Span::styled(value.clone(), value_style()),
            ])
        })
        .collect();

    let max_summary_rows = summary_lines.len().max(technical_lines.len());
    let mut summary_height = max_summary_rows.saturating_add(2) as u16;
    let max_height = area.height.max(1u16);
    if summary_height > max_height {
        summary_height = max_height;
    }
    let min_required = 3u16.min(max_height);
    if summary_height < min_required {
        summary_height = min_required;
    }

    let detail_mode = s.history.detail_mode;
    let mut sorted_rows = record.rows.clone();
    sort_rows_for_mode(&mut sorted_rows, detail_mode);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Min(6),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);

    let summary_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(layout[0]);

    let summary = Paragraph::new(summary_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![Span::styled(
                    format!("Details · {}", encounter.display_title),
                    title_style(),
                )])),
        )
        .alignment(Alignment::Left);
    f.render_widget(summary, summary_chunks[0]);

    let technical = Paragraph::new(technical_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![Span::styled(
                    "Technical Details".to_string(),
                    title_style(),
                )])),
        )
        .alignment(Alignment::Left);
    f.render_widget(technical, summary_chunks[1]);

    if sorted_rows.is_empty() {
        let block = Paragraph::new("No combatants recorded.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, layout[1]);
    } else {
        let table_title = Line::from(vec![
            Span::styled(
                format!("Combatants · {}", detail_mode.label()),
                title_style(),
            ),
            Span::raw(" "),
            Span::styled("(m toggles)", Style::default().fg(TEXT)),
        ]);
        let block = Block::default().borders(Borders::ALL).title(table_title);
        let table_area = layout[1];
        let inner = block.inner(table_area);
        f.render_widget(block, table_area);

        let ctx = TableRenderContext {
            rows: &sorted_rows,
            mode: detail_mode,
            decoration: s.decoration,
        };
        draw_table_with_context(f, inner, &ctx);
    }

    let metric_label = match detail_mode {
        ViewMode::Dps => "ENCDPS",
        ViewMode::Heal => "ENCHPS",
    };
    let metric_value = match detail_mode {
        ViewMode::Dps => &record.encounter.encdps,
        ViewMode::Heal => &record.encounter.enchps,
    };
    let total_label = match detail_mode {
        ViewMode::Dps => "Total Damage",
        ViewMode::Heal => "Total Healed",
    };
    let total_value = match detail_mode {
        ViewMode::Dps => &record.encounter.damage,
        ViewMode::Heal => &record.encounter.healed,
    };

    let metric_value = if metric_value.is_empty() {
        "—".to_string()
    } else {
        metric_value.clone()
    };
    let total_value = if total_value.is_empty() {
        "—".to_string()
    } else {
        total_value.clone()
    };

    let mode_lines = vec![
        Line::from(vec![
            Span::styled("Current: ", header_style()),
            Span::styled(detail_mode.label(), value_style()),
            Span::styled(" · press m to toggle", Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Sorting: ", header_style()),
            Span::styled(metric_label, value_style()),
            Span::styled(" · encounter ", Style::default().fg(TEXT)),
            Span::styled(metric_label, value_style()),
            Span::styled(": ", Style::default().fg(TEXT)),
            Span::styled(metric_value, value_style()),
            Span::styled(" · ", Style::default().fg(TEXT)),
            Span::styled(total_label, header_style()),
            Span::styled(": ", Style::default().fg(TEXT)),
            Span::styled(total_value, value_style()),
        ]),
    ];

    let mode_paragraph = Paragraph::new(mode_lines).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(vec![Span::styled("View Mode", title_style())])),
    );
    f.render_widget(mode_paragraph, layout[2]);

    let hint = Paragraph::new("← back · ↑/↓ switch encounter · m toggles DPS/Heal · Enter re-open")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, layout[3]);
}

fn draw_dungeon_dates(f: &mut Frame, area: Rect, s: &AppSnapshot) {
    let items: Vec<ListItem> = s
        .history
        .dungeon_days
        .iter()
        .map(|day| ListItem::new(day.label.clone()))
        .collect();

    let mut state = ListState::default();
    state.select(Some(s.history.dungeon_selected_day));

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
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[0], &mut state);

    let hint = Paragraph::new("Tab swaps view · Enter view runs")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, chunks[1]);
}

fn draw_dungeon_runs(f: &mut Frame, area: Rect, s: &AppSnapshot) {
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

    let mut state = ListState::default();
    state.select(Some(s.history.dungeon_selected_run));

    let title = format!("Dungeon Runs · {}", day.label);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_dungeon_run_detail(f: &mut Frame, area: Rect, s: &AppSnapshot) {
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
            format_duration_short(record.total_duration_secs),
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
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    let mut list_items = Vec::new();
    let metric_label = match detail_mode {
        ViewMode::Dps => "DPS",
        ViewMode::Heal => "HPS",
    };

    for (idx, title) in record.child_titles.iter().enumerate() {
        let label = if let Some(child) = run.child_records.get(idx).and_then(|c| c.as_ref()) {
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

    let mut list_state = ListState::default();
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
        f.render_stateful_widget(list, layout[1], &mut list_state);
    }

    let instructions = Paragraph::new("← runs · ↑/↓ select pull · Enter view pull · m toggles DPS/Heal")
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

    let Some(encounter_record) = run.child_records.get(idx).and_then(|c| c.as_ref()) else {
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

    let detail_mode = s.history.detail_mode;
    let mut sorted_rows = encounter_record.rows.clone();
    sort_rows_for_mode(&mut sorted_rows, detail_mode);

    let basic_metrics = [
        (
            "Encounter",
            if encounter_record.encounter.title.is_empty() {
                title.clone()
            } else {
                encounter_record.encounter.title.clone()
            },
        ),
        (
            "Zone",
            if encounter_record.encounter.zone.is_empty() {
                run.zone.clone()
            } else {
                encounter_record.encounter.zone.clone()
            },
        ),
        ("Duration", encounter_record.encounter.duration.clone()),
        ("ENCDPS", encounter_record.encounter.encdps.clone()),
        ("Damage", encounter_record.encounter.damage.clone()),
    ];

    let technical_metrics = [
        ("Snapshots", encounter_record.snapshots.to_string()),
        ("Frames", encounter_record.frames.len().to_string()),
        (
            "Last seen",
            format_timestamp_label(encounter_record.last_seen_ms),
        ),
    ];

    let summary_lines: Vec<Line> = basic_metrics
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(format!("{label}: "), header_style()),
                Span::styled(value.clone(), value_style()),
            ])
        })
        .collect();

    let technical_lines: Vec<Line> = technical_metrics
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(format!("{label}: "), header_style()),
                Span::styled(value.clone(), value_style()),
            ])
        })
        .collect();

    let max_summary_rows = summary_lines.len().max(technical_lines.len());
    let mut summary_height = max_summary_rows.saturating_add(2) as u16;
    let max_height = area.height.max(1u16);
    if summary_height > max_height {
        summary_height = max_height;
    }
    let min_required = 3u16.min(max_height);
    if summary_height < min_required {
        summary_height = min_required;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Min(6),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);

    let summary_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(layout[0]);

    let summary = Paragraph::new(summary_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![Span::styled(
                    format!("Details · {title}"),
                    title_style(),
                )])),
        )
        .alignment(Alignment::Left);
    f.render_widget(summary, summary_chunks[0]);

    let technical = Paragraph::new(technical_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![Span::styled(
                    "Technical Details".to_string(),
                    title_style(),
                )])),
        )
        .alignment(Alignment::Left);
    f.render_widget(technical, summary_chunks[1]);

    if sorted_rows.is_empty() {
        let block = Paragraph::new("No combatants recorded.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(block, layout[1]);
    } else {
        let table_title = Line::from(vec![
            Span::styled(
                format!("Combatants · {}", detail_mode.label()),
                title_style(),
            ),
            Span::raw(" "),
            Span::styled("(m toggles)", Style::default().fg(TEXT)),
        ]);
        let block = Block::default().borders(Borders::ALL).title(table_title);
        let table_area = layout[1];
        let inner = block.inner(table_area);
        f.render_widget(block, table_area);

        let ctx = TableRenderContext {
            rows: &sorted_rows,
            mode: detail_mode,
            decoration: s.decoration,
        };
        draw_table_with_context(f, inner, &ctx);
    }

    let metric_label = match detail_mode {
        ViewMode::Dps => "ENCDPS",
        ViewMode::Heal => "ENCHPS",
    };
    let metric_value = match detail_mode {
        ViewMode::Dps => &encounter_record.encounter.encdps,
        ViewMode::Heal => &encounter_record.encounter.enchps,
    };
    let total_label = match detail_mode {
        ViewMode::Dps => "Total Damage",
        ViewMode::Heal => "Total Healed",
    };
    let total_value = match detail_mode {
        ViewMode::Dps => &encounter_record.encounter.damage,
        ViewMode::Heal => &encounter_record.encounter.healed,
    };

    let metric_value = if metric_value.is_empty() {
        "—".to_string()
    } else {
        metric_value.clone()
    };
    let total_value = if total_value.is_empty() {
        "—".to_string()
    } else {
        total_value.clone()
    };

    let mode_lines = vec![
        Line::from(vec![
            Span::styled("Current: ", header_style()),
            Span::styled(detail_mode.label(), value_style()),
            Span::styled(" · press m to toggle", Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Sorting: ", header_style()),
            Span::styled(metric_label, value_style()),
            Span::styled(" · encounter ", Style::default().fg(TEXT)),
            Span::styled(metric_label, value_style()),
            Span::styled(": ", Style::default().fg(TEXT)),
            Span::styled(metric_value, value_style()),
            Span::styled(" · ", Style::default().fg(TEXT)),
            Span::styled(total_label, header_style()),
            Span::styled(": ", Style::default().fg(TEXT)),
            Span::styled(total_value, value_style()),
        ]),
    ];

    let mode_paragraph = Paragraph::new(mode_lines).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(vec![Span::styled("View Mode", title_style())])),
    );
    f.render_widget(mode_paragraph, layout[2]);

    let hint =
        Paragraph::new("← run detail · ↑/↓ switch pull · m toggles DPS/Heal · Enter re-open")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, layout[3]);
}

fn sort_rows_for_mode(rows: &mut [CombatantRow], mode: ViewMode) {
    match mode {
        ViewMode::Dps => rows.sort_by(|a, b| {
            b.encdps
                .partial_cmp(&a.encdps)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        ViewMode::Heal => rows.sort_by(|a, b| {
            b.enchps
                .partial_cmp(&a.enchps)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
    }
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

fn format_duration_short(total_secs: u64) -> String {
    if total_secs == 0 {
        return "00:00".to_string();
    }
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

fn format_number(value: f64) -> String {
    if value.abs() >= 1000.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
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
    sig.iter().cloned().collect::<Vec<_>>().join(", ")
}
