use std::cmp::Ordering;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::history::types::EncounterRecord;
use crate::model::{CombatantRow, Decoration, LimitBreakMode, LimitBreakSummary, ViewMode};
use crate::theme::{header_style, text_color, title_style, value_style};
use crate::ui::lb::{self, LB_PANEL_HEIGHT};
use crate::ui::{draw_table_with_context, TableRenderContext};

pub(crate) struct EncounterDetailParams<'a> {
    pub record: &'a EncounterRecord,
    pub title: String,
    pub zone_fallback: String,
    pub last_seen_label: String,
    pub detail_mode: ViewMode,
    pub decoration: Decoration,
    pub limit_break_mode: LimitBreakMode,
    pub footer_hint: &'static str,
}

pub(crate) fn draw_encounter_record(f: &mut Frame, area: Rect, params: EncounterDetailParams<'_>) {
    let EncounterDetailParams {
        record,
        title,
        zone_fallback,
        last_seen_label,
        detail_mode,
        decoration,
        limit_break_mode,
        footer_hint,
    } = params;

    let basic_metrics = [
        (
            "Encounter",
            if record.encounter.title.is_empty() {
                title.clone()
            } else {
                record.encounter.title.clone()
            },
        ),
        (
            "Zone",
            if record.encounter.zone.is_empty() {
                zone_fallback
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
        ("Last seen", last_seen_label),
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

    let mut sorted_rows = record.rows.clone();
    let show_panel = limit_break_mode == LimitBreakMode::Panel;
    let show_table_row = limit_break_mode == LimitBreakMode::TableRow;

    if show_table_row {
        if let Some(lb) = record.lb_summary.as_ref() {
            lb::inject_lb_table_row(&mut sorted_rows, lb, &record.encounter, detail_mode);
        } else {
            sort_rows_for_mode(&mut sorted_rows, detail_mode);
        }
    } else {
        sort_rows_for_mode(&mut sorted_rows, detail_mode);
    }

    let constraints = if show_panel {
        vec![
            Constraint::Length(summary_height),
            Constraint::Min(6),
            Constraint::Length(4),
            Constraint::Length(LB_PANEL_HEIGHT),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(summary_height),
            Constraint::Min(6),
            Constraint::Length(4),
            Constraint::Length(1),
        ]
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
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
            Span::styled("(m toggles)", Style::default().fg(text_color())),
        ]);
        let block = Block::default().borders(Borders::ALL).title(table_title);
        let table_area = layout[1];
        let inner = block.inner(table_area);
        f.render_widget(block, table_area);

        let ctx = TableRenderContext {
            rows: &sorted_rows,
            mode: detail_mode,
            decoration,
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
            Span::styled(" · press m to toggle", Style::default().fg(text_color())),
        ]),
        Line::from(vec![
            Span::styled("Sorting: ", header_style()),
            Span::styled(metric_label, value_style()),
            Span::styled(" · encounter ", Style::default().fg(text_color())),
            Span::styled(metric_label, value_style()),
            Span::styled(": ", Style::default().fg(text_color())),
            Span::styled(metric_value, value_style()),
            Span::styled(" · ", Style::default().fg(text_color())),
            Span::styled(total_label, header_style()),
            Span::styled(": ", Style::default().fg(text_color())),
            Span::styled(total_value, value_style()),
        ]),
    ];

    let mode_paragraph = Paragraph::new(mode_lines).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(vec![Span::styled("View Mode", title_style())])),
    );
    f.render_widget(mode_paragraph, layout[2]);

    let hint_idx = if show_panel {
        let placeholder = LimitBreakSummary {
            user: "—".to_string(),
            damage: 0,
        };
        let lb_ref = record.lb_summary.as_ref().unwrap_or(&placeholder);
        lb::draw(f, layout[3], lb_ref);
        4
    } else {
        3
    };

    let hint = Paragraph::new(footer_hint)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(hint, layout[hint_idx]);
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
