use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::model::{AppSnapshot, SettingsField};
use crate::theme::{header_style, theme_registry, title_style, value_style};

pub(super) fn draw(f: &mut Frame, snapshot: &AppSnapshot) {
    let area = centered_rect(60, 50, f.size());
    f.render_widget(Clear, area);

    let idle_selected = matches!(snapshot.settings_cursor, SettingsField::IdleTimeout);
    let decor_selected = matches!(snapshot.settings_cursor, SettingsField::DefaultDecoration);
    let mode_selected = matches!(snapshot.settings_cursor, SettingsField::DefaultMode);
    let dungeon_selected = matches!(snapshot.settings_cursor, SettingsField::DungeonMode);
    let limit_break_selected = matches!(snapshot.settings_cursor, SettingsField::LimitBreakMode);
    let theme_selected = matches!(snapshot.settings_cursor, SettingsField::Theme);
    let role_theme_selected = matches!(snapshot.settings_cursor, SettingsField::RoleTheme);

    let mut lines = Vec::new();
    //lines.push(Line::from(vec![Span::styled("Settings", title_style())]));
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
    lines.push(setting_line(
        limit_break_selected,
        "Limit break display",
        limit_break_mode_label(snapshot.settings.limit_break_mode),
    ));
    lines.push(setting_line(
        theme_selected,
        "Theme",
        current_theme_name(&snapshot.settings.theme_id),
    ));
    lines.push(setting_line(
        role_theme_selected,
        "Change role specific colors",
        if snapshot.settings.role_theme_enabled {
            "ON".to_string()
        } else {
            "OFF".to_string()
        },
    ));
    lines.push(Line::default());

    lines.push(Line::from(vec![Span::styled(
        "Use ↑/↓ to select, ←/→ to adjust.",
        header_style(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "Press 'q' or 's' to close.",
        header_style(),
    )]));
    lines.push(Line::default());

    // Calculate content height (lines + block borders)
    let content_height = lines.len() as u16 + 2; // +2 for top and bottom borders
    let available_height = area.height;
    
    // Center the content vertically
    let top_padding = if available_height > content_height {
        (available_height - content_height) / 2
    } else {
        0
    };
    let bottom_padding = if available_height > content_height {
        available_height - content_height - top_padding
    } else {
        0
    };
    
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_padding),
            Constraint::Length(content_height.min(available_height)),
            Constraint::Length(bottom_padding),
        ])
        .split(area);
    
    let content_area = vertical_layout[1];

    let block = Block::default()
        .title(Line::from(vec![Span::styled("Settings", title_style())]))
        .borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(widget, content_area);
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

fn current_theme_name(theme_id: &str) -> String {
    let registry = theme_registry();
    if let Some(desc) = registry
        .descriptors()
        .into_iter()
        .find(|d| d.id.eq_ignore_ascii_case(theme_id))
    {
        desc.name
    } else {
        let default_id = registry.default_id();
        registry
            .descriptors()
            .into_iter()
            .find(|d| d.id == default_id)
            .map(|d| d.name)
            .unwrap_or_else(|| "Synth Wave".to_string())
    }
}

fn limit_break_mode_label(mode: u8) -> String {
    match mode {
        0 => "OFF".to_string(),
        1 => "PANEL".to_string(),
        2 => "TABLE".to_string(),
        _ => "OFF".to_string(),
    }
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
