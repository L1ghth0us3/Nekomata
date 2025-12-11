use std::borrow::Cow;

use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::errors::AppError;
use crate::model::AppSnapshot;
use crate::theme::{header_style, title_style, value_style};

pub(super) fn draw(f: &mut Frame, area: ratatui::layout::Rect, snapshot: &AppSnapshot) {
    let (status_text, status_style) = status_label(snapshot);
    let status_span = Span::styled(status_text.clone(), status_style);

    let (dungeon_text, dungeon_style) = dungeon_label(snapshot);
    let dungeon_span = Span::styled(dungeon_text, dungeon_style);

    let decor_label = snapshot
        .decoration
        .short_label()
        .trim_start_matches("decor:");
    let mode_label = snapshot.mode.short_label().trim_start_matches("mode:");
    let history_style = if snapshot.history.visible {
        header_style().add_modifier(Modifier::BOLD)
    } else {
        header_style()
    };

    let width = area.width as usize;
    let line = footer_line(
        width,
        status_span,
        dungeon_span,
        decor_label,
        mode_label,
        history_style,
    );

    let widget = Paragraph::new(line)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Left);
    f.render_widget(widget, area);
}

pub(super) fn draw_error(f: &mut Frame, area: ratatui::layout::Rect, error: &AppError) {
    let label = error.kind().label();
    let summary = error.summary_line();
    let text = format!("{label} error: {summary}. Run with --debug for details.");

    let widget = Paragraph::new(Line::from(Span::raw(text)))
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Left)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(crate::theme::STATUS_DISCONNECTED)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(widget, area);
}

fn status_label(snapshot: &AppSnapshot) -> (Cow<'static, str>, Style) {
    if !snapshot.connected {
        (
            Cow::Borrowed("Disconnected"),
            Style::default().fg(crate::theme::STATUS_DISCONNECTED),
        )
    } else if snapshot.is_idle {
        (
            Cow::Borrowed("Connected (idle)"),
            Style::default().fg(crate::theme::STATUS_IDLE),
        )
    } else {
        (Cow::Borrowed("Connected"), value_style())
    }
}

fn dungeon_label(snapshot: &AppSnapshot) -> (String, Style) {
    if !snapshot.settings.dungeon_mode_enabled {
        ("Dungeon: Off".to_string(), header_style())
    } else if let Some(zone) = snapshot.dungeon_active_zone.as_ref() {
        (format!("Dungeon: {zone}"), value_style())
    } else {
        ("Dungeon: On".to_string(), header_style())
    }
}

fn footer_line(
    width: usize,
    status_span: Span<'static>,
    dungeon_span: Span<'static>,
    decor_label: &str,
    mode_label: &str,
    history_style: Style,
) -> Line<'static> {
    if width >= 90 {
        Line::from(vec![
            Span::styled(" q ", title_style()),
            Span::styled("quit", header_style()),
            Span::raw(" | "),
            Span::styled(" m ", title_style()),
            Span::styled(mode_label.to_string(), header_style()),
            Span::raw(" | "),
            Span::styled(" s ", title_style()),
            Span::styled("settings", header_style()),
            Span::raw(" | "),
            Span::styled(" h ", title_style()),
            Span::styled("history", history_style),
            Span::raw(" | "),
            Span::styled(" d ", title_style()),
            Span::styled(decor_label.to_string(), header_style()),
            Span::raw(" | "),
            Span::styled(" view ", title_style()),
            dungeon_span.clone(),
            Span::raw(" | "),
            Span::styled("status", header_style()),
            Span::raw(" "),
            status_span.clone(),
        ])
    } else if width >= 60 {
        Line::from(vec![
            Span::styled(" q ", title_style()),
            Span::styled("quit", header_style()),
            Span::raw(" | "),
            Span::styled(" m ", title_style()),
            Span::styled(mode_label.to_string(), header_style()),
            Span::raw(" | "),
            Span::styled(" s ", title_style()),
            Span::styled("settings", header_style()),
            Span::raw(" | "),
            Span::styled(" h ", title_style()),
            Span::styled("history", history_style),
            Span::raw(" | "),
            Span::styled(" d ", title_style()),
            Span::styled(decor_label.to_string(), header_style()),
            Span::raw(" | "),
            dungeon_span.clone(),
            Span::raw(" | "),
            status_span,
        ])
    } else if width >= 36 {
        Line::from(vec![
            Span::styled(" q ", title_style()),
            Span::styled(" m ", title_style()),
            Span::styled(" s ", title_style()),
            Span::styled(" h ", title_style()),
            Span::styled(" d ", title_style()),
            dungeon_span,
            status_span,
        ])
    } else {
        Line::from(vec![Span::styled("qmshd", title_style())])
    }
}
