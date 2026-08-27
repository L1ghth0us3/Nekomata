use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::history::{format_bytes, HistoryLimitKind};
use crate::model::{
    AppSnapshot, ConfirmFocus, HistorySettingsField, HistorySettingsPanel,
};
use crate::theme::{header_style, title_style, value_style};

pub(super) fn draw(f: &mut Frame, snapshot: &AppSnapshot) {
    let panel = &snapshot.history_settings;
    if !panel.visible {
        return;
    }

    if let Some(confirm) = &panel.confirm {
        draw_confirm(f, confirm);
        return;
    }
    if let Some(prompt) = &panel.filename_prompt {
        draw_filename_prompt(f, prompt);
        return;
    }
    if let Some(browser) = &panel.archive_browser {
        draw_archive_browser(f, browser);
        return;
    }

    draw_main_panel(f, snapshot, panel);
}

fn draw_main_panel(f: &mut Frame, snapshot: &AppSnapshot, panel: &HistorySettingsPanel) {
    let area = centered_rect(70, 60, f.size());
    f.render_widget(Clear, area);

    let mut lines = Vec::new();
    lines.push(Line::default());

    let size_label = match panel.live_db_size {
        Some(bytes) => format!("Live DB: {}", format_bytes(bytes)),
        None if snapshot.settings.history_enabled => "Live DB: —".to_string(),
        None => "Live DB: disconnected".to_string(),
    };
    lines.push(Line::from(vec![Span::styled(size_label, header_style())]));
    if let Some(msg) = &panel.status_message {
        for line in msg.lines() {
            lines.push(Line::from(vec![Span::styled(line.to_string(), value_style())]));
        }
    }
    lines.push(Line::default());

    let recording_selected = panel.cursor == HistorySettingsField::Recording;
    lines.push(setting_line(
        recording_selected,
        "History recording",
        if snapshot.settings.history_enabled {
            "ON".to_string()
        } else {
            "OFF".to_string()
        },
    ));

    let kind_selected = panel.cursor == HistorySettingsField::LimitKind;
    let kind_suffix = if panel.has_draft_changes() {
        " (not applied)".to_string()
    } else {
        String::new()
    };
    lines.push(setting_line(
        kind_selected,
        &format!("History limit{kind_suffix}"),
        panel.draft_limit.kind.label().to_string(),
    ));

    if panel.draft_limit.kind != HistoryLimitKind::None {
        let value_selected = panel.cursor == HistorySettingsField::LimitValue;
        let (label, value) = match panel.draft_limit.kind {
            HistoryLimitKind::MaxAgeDays => (
                "Limit value",
                format!("{} days", panel.draft_limit.days),
            ),
            HistoryLimitKind::MaxSizeMb => (
                "Limit value",
                format!("{} MB", panel.draft_limit.size_mb),
            ),
            HistoryLimitKind::None => ("Limit value", "—".to_string()),
        };
        lines.push(setting_line(
            value_selected,
            label,
            value,
        ));
    }

    lines.push(Line::from(vec![Span::styled(
        "────────",
        Style::default().add_modifier(Modifier::DIM),
    )]));

    lines.push(action_line(
        panel.cursor == HistorySettingsField::CreateBackup,
        "Create backup…",
        "Enter".to_string(),
    ));
    let archives_empty = snapshot.archive_count == 0;
    lines.push(action_line(
        panel.cursor == HistorySettingsField::BrowseArchives,
        "Browse archives…",
        if archives_empty {
            "No archives yet".to_string()
        } else {
            "Enter".to_string()
        },
    ));
    lines.push(action_line(
        panel.cursor == HistorySettingsField::DeleteCurrent,
        "Delete current history…",
        "Enter".to_string(),
    ));

    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "↑/↓ select · ←/→ adjust · Enter activate · Esc back",
        header_style(),
    )]));
    if panel.has_draft_changes() {
        lines.push(Line::from(vec![Span::styled(
            "Enter on limit row to apply changes",
            header_style(),
        )]));
    }

    let content_height = lines.len() as u16 + 2;
    let top_padding = area.height.saturating_sub(content_height) / 2;
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_padding),
            Constraint::Length(content_height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "History Settings",
            title_style(),
        )]))
        .borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(widget, vertical[1]);
}

fn draw_confirm(f: &mut Frame, confirm: &crate::model::ConfirmDialog) {
    let area = centered_rect(65, 40, f.size());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::default()];
    for paragraph in confirm.message.lines() {
        lines.push(Line::from(vec![Span::raw(paragraph.to_string())]));
    }
    lines.push(Line::default());
    let cancel_style = if confirm.focus == ConfirmFocus::Cancel {
        title_style()
    } else {
        header_style()
    };
    let confirm_style = if confirm.focus == ConfirmFocus::Confirm {
        title_style()
    } else {
        header_style()
    };
    lines.push(Line::from(vec![
        Span::styled(
            format!(
                "{} Cancel",
                if confirm.focus == ConfirmFocus::Cancel {
                    "▶"
                } else {
                    " "
                }
            ),
            cancel_style,
        ),
        Span::raw("   "),
        Span::styled(
            format!(
                "{} {}",
                if confirm.focus == ConfirmFocus::Confirm {
                    "▶"
                } else {
                    " "
                },
                confirm.confirm_label
            ),
            confirm_style,
        ),
    ]));
    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "←/→ choose · Enter confirm · Esc cancel",
        header_style(),
    )]));

    let block = Block::default()
        .title(Line::from(vec![Span::styled("Confirm", title_style())]))
        .borders(Borders::ALL);
    f.render_widget(
        Paragraph::new(lines).block(block).alignment(Alignment::Center),
        area,
    );
}

fn draw_filename_prompt(f: &mut Frame, prompt: &crate::model::FilenamePrompt) {
    let area = centered_rect(60, 30, f.size());
    f.render_widget(Clear, area);

    let mut lines = vec![
        Line::default(),
        Line::from(vec![Span::styled(&prompt.title, header_style())]),
        Line::default(),
        Line::from(vec![
            Span::raw("Name: "),
            Span::styled(&prompt.value, value_style()),
            Span::styled("_", value_style()),
        ]),
    ];
    if let Some(err) = &prompt.error {
        lines.push(Line::from(vec![Span::styled(err.clone(), value_style())]));
    }
    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "Type name · Enter save · Esc cancel",
        header_style(),
    )]));

    let block = Block::default()
        .title(Line::from(vec![Span::styled("Backup", title_style())]))
        .borders(Borders::ALL);
    f.render_widget(
        Paragraph::new(lines).block(block).alignment(Alignment::Center),
        area,
    );
}

fn draw_archive_browser(f: &mut Frame, browser: &crate::model::ArchiveBrowser) {
    let area = centered_rect(70, 55, f.size());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::default()];
    if browser.entries.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No archives yet. Create a backup first.",
            header_style(),
        )]));
    } else {
        for (idx, entry) in browser.entries.iter().enumerate() {
            let selected = idx == browser.selected;
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} {}", if selected { "▶" } else { " " }, entry.name),
                    if selected {
                        title_style()
                    } else {
                        header_style()
                    },
                ),
                Span::raw(" "),
                Span::styled(format_bytes(entry.size_bytes), value_style()),
            ]));
        }
    }
    if let Some(err) = &browser.error {
        lines.push(Line::from(vec![Span::styled(err.clone(), value_style())]));
    }
    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "Enter view · d delete · Esc back",
        header_style(),
    )]));

    let block = Block::default()
        .title(Line::from(vec![Span::styled("Archives", title_style())]))
        .borders(Borders::ALL);
    f.render_widget(
        Paragraph::new(lines).block(block).alignment(Alignment::Center),
        area,
    );
}

fn setting_line(selected: bool, label: &str, value: String) -> Line<'static> {
    let marker = if selected { "▶" } else { " " };
    let label_style = if selected {
        title_style()
    } else {
        header_style()
    };
    Line::from(vec![
        Span::styled(format!("{} {}:", marker, label), label_style),
        Span::raw(" "),
        Span::styled(value, value_style()),
    ])
}

fn action_line(selected: bool, label: &str, hint: String) -> Line<'static> {
    let marker = if selected { "▶" } else { " " };
    let label_style = if selected {
        title_style()
    } else {
        header_style()
    };
    Line::from(vec![
        Span::styled(format!("{} {}", marker, label), label_style),
        Span::raw("  "),
        Span::styled(hint, value_style()),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(area);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(horizontal[1]);
    vertical[1]
}
