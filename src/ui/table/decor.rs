use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::TableRenderContext;
use crate::model::{CombatantRow, ViewMode};
use crate::theme::role_bar_color;

fn metric_for_mode(mode: ViewMode, row: &CombatantRow) -> f64 {
    match mode {
        ViewMode::Dps => row.encdps,
        ViewMode::Heal => row.enchps,
    }
}

pub(super) fn draw_background_meters(
    f: &mut Frame,
    area: Rect,
    ctx: &TableRenderContext<'_>,
    header_lines: u16,
) {
    if area.height <= header_lines {
        return;
    }

    let max_metric = ctx
        .rows
        .iter()
        .map(|r| metric_for_mode(ctx.mode, r))
        .fold(0.0_f64, |a, b| a.max(b));
    if max_metric <= 0.0 {
        return;
    }

    let width = area.width as usize;
    let visible_rows = (area.height.saturating_sub(header_lines)) as usize;

    for (index, row) in ctx.rows.iter().take(visible_rows).enumerate() {
        let ratio = (metric_for_mode(ctx.mode, row) / max_metric).clamp(0.0, 1.0);
        let filled = (ratio * width as f64).round() as usize;
        let y = area.y + header_lines + index as u16;
        if y >= area.y + area.height {
            break;
        }

        let rect = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };

        let mut spans = Vec::with_capacity(2);
        if filled > 0 {
            spans.push(Span::styled(
                " ".repeat(filled),
                Style::default().bg(role_bar_color(&row.job)),
            ));
        }
        if width > filled {
            spans.push(Span::raw(" ".repeat(width - filled)));
        }

        let bg = Paragraph::new(Line::from(spans));
        f.render_widget(bg, rect);
    }
}

pub(super) fn draw_underlines(
    f: &mut Frame,
    area: Rect,
    ctx: &TableRenderContext<'_>,
    header_lines: u16,
) {
    if area.height <= header_lines {
        return;
    }

    let max_metric = ctx
        .rows
        .iter()
        .map(|r| metric_for_mode(ctx.mode, r))
        .fold(0.0_f64, |a, b| if b > a { b } else { a });
    if max_metric <= 0.0 {
        return;
    }

    let usable_height = area.height.saturating_sub(header_lines);
    let visible_rows = (usable_height / 2) as usize;
    let width = area.width as usize;

    for (index, row) in ctx.rows.iter().take(visible_rows).enumerate() {
        let ratio = (metric_for_mode(ctx.mode, row) / max_metric).clamp(0.0, 1.0);
        let filled = (ratio * width as f64).round() as usize;
        let y = area.y + header_lines + (index as u16) * 2 + 1;
        if y >= area.y + area.height {
            break;
        }

        let rect = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };

        let mut line = String::with_capacity(width);
        for _ in 0..filled {
            line.push('▔');
        }
        for _ in filled..width {
            line.push(' ');
        }

        let para = Paragraph::new(Line::from(Span::styled(
            line,
            Style::default().fg(role_bar_color(&row.job)),
        )));

        f.render_widget(para, rect);
    }
}
