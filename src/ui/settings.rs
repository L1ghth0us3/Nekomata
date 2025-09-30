use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::model::{AppSnapshot, SettingsField};
use crate::theme::{header_style, title_style, value_style};

pub(super) fn draw(f: &mut Frame, snapshot: &AppSnapshot) {
    let area = centered_rect(60, 50, f.size());
    f.render_widget(Clear, area);

    let idle_selected = matches!(snapshot.settings_cursor, SettingsField::IdleTimeout);
    let decor_selected = matches!(snapshot.settings_cursor, SettingsField::DefaultDecoration);
    let mode_selected = matches!(snapshot.settings_cursor, SettingsField::DefaultMode);
    let dungeon_selected = matches!(snapshot.settings_cursor, SettingsField::DungeonMode);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled("Settings", title_style())]));
    lines.push(Line::default());

    lines.push(setting_line(
        idle_selected,
        "Idle timeout",
        format!("{}s", snapshot.settings.idle_seconds),
    ));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled("Set to 0 to disable idle mode.", header_style()),
    ]));
    lines.push(Line::default());

    lines.push(setting_line(
        decor_selected,
        "Default decoration",
        snapshot.settings.default_decoration.label().to_string(),
    ));
    lines.push(setting_line(
        mode_selected,
        "Default mode",
        snapshot.settings.default_mode.label().to_string(),
    ));
    lines.push(setting_line(
        dungeon_selected,
        "Dungeon Mode",
        if snapshot.settings.dungeon_mode_enabled {
            "ON".to_string()
        } else {
            "OFF".to_string()
        },
    ));
    lines.push(Line::default());

    lines.push(Line::from(vec![Span::styled(
        "Use ↑/↓ to select, ←/→ to adjust. Press 's' to close.",
        header_style(),
    )]));

    let block = Block::default().title("Settings").borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(widget, area);
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
