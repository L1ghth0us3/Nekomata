use ratatui::layout::Alignment;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::model::LimitBreakSummary;
use crate::theme::{header_style, title_style, value_style};

/// Min vertical space for borders (2) + two text lines (2).
pub(crate) const LB_PANEL_HEIGHT: u16 = 3;

pub(crate) fn draw(f: &mut Frame, area: ratatui::layout::Rect, lb: &LimitBreakSummary) {
    // Content area height is 1 line (LB_PANEL_HEIGHT=3 and borders=ALL),
    // so ratatui will naturally clip instead of wrapping.
    let content_line = Line::from(vec![
        Span::styled("Name: ", header_style()),
        Span::styled(lb.user.clone(), value_style()),
        Span::raw("  "),
        Span::styled("Damage: ", header_style()),
        Span::styled(lb.damage.to_string(), value_style()),
    ]);

    // Match the table's header separator line color.
    let border_style = ratatui::style::Style::default().fg(Color::Rgb(170, 170, 180));

    let widget = Paragraph::new(content_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Line::from(vec![Span::styled("Limit Break", title_style())])),
        )
        .alignment(Alignment::Left);
    f.render_widget(widget, area);
}
