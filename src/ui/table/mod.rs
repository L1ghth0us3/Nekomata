use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Table};
use ratatui::Frame;

use crate::model::{AppSnapshot, CombatantRow, Decoration, ViewMode};

mod decor;
mod layout;

pub(super) fn draw(f: &mut Frame, area: Rect, snapshot: &AppSnapshot) {
    let ctx = TableRenderContext {
        rows: &snapshot.rows,
        mode: snapshot.mode,
        decoration: snapshot.decoration,
    };
    draw_with_context(f, area, &ctx);
}

#[derive(Clone, Copy)]
pub(crate) struct TableRenderContext<'a> {
    pub rows: &'a [CombatantRow],
    pub mode: ViewMode,
    pub decoration: Decoration,
}

pub(crate) fn draw_with_context(f: &mut Frame, area: Rect, ctx: &TableRenderContext<'_>) {
    f.render_widget(Clear, area);

    let width = area.width as usize;
    let row_height = ctx.decoration.row_height();
    let layout = layout::layout_for(ctx.mode, width);
    let header_lines = layout.header_height();

    if matches!(ctx.decoration, Decoration::Background) {
        decor::draw_background_meters(f, area, ctx, header_lines);
    }

    let table = Table::new(
        ctx.rows.iter().map(|row| layout.data_row(row, row_height)),
        layout.widths(),
    )
    .header(layout.header_row())
    .block(Block::default().borders(Borders::NONE))
    .column_spacing(layout.column_spacing());

    f.render_widget(table, area);

    if area.height > header_lines && header_lines > 0 {
        draw_header_separator(f, area, header_lines);
    }

    if matches!(ctx.decoration, Decoration::Underline) {
        decor::draw_underlines(f, area, ctx, header_lines);
    }
}

fn draw_header_separator(f: &mut Frame, area: Rect, header_lines: u16) {
    let sep_offset = header_lines.saturating_sub(1);
    let sep_y = area.y.saturating_add(sep_offset);
    if sep_y >= area.y + area.height {
        return;
    }

    let width = area.width as usize;
    let line = "─".repeat(width);
    let rect = Rect {
        x: area.x,
        y: sep_y,
        width: area.width,
        height: 1,
    };
    let separator = Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(Color::Rgb(170, 170, 180)),
    )));
    f.render_widget(separator, rect);
}
