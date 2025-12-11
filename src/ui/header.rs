use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::model::{AppSnapshot, ViewMode};
use crate::theme::{header_style, value_style, TEXT};

pub(super) fn draw(f: &mut Frame, area: Rect, snapshot: &AppSnapshot) {
    let block = Block::default().borders(Borders::NONE);
    let width = area.width as usize;

    let top_line = header_metrics_line(snapshot, width);
    let bottom_line = header_title_line(snapshot, width);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
    let top_area = chunks[0];
    let bottom_area = chunks[1];

    let top_widget = Paragraph::new(bottom_line)
        .block(block.clone())
        .style(Style::default().fg(TEXT))
        .alignment(Alignment::Left);
    f.render_widget(top_widget, top_area);

    let bottom_widget = Paragraph::new(top_line)
        .block(block)
        .style(Style::default().fg(TEXT))
        .alignment(Alignment::Left);
    f.render_widget(bottom_widget, bottom_area);
}

fn header_metrics_line(snapshot: &AppSnapshot, width: usize) -> Line<'static> {
    if let Some(enc) = &snapshot.encounter {
        let (metric_label, metric_val, total_label, total_val) = match snapshot.mode {
            ViewMode::Dps => ("ENCDPS", enc.encdps.as_str(), "Damage", enc.damage.as_str()),
            ViewMode::Heal => ("ENCHPS", enc.enchps.as_str(), "Healed", enc.healed.as_str()),
        };

        if width >= 56 {
            Line::from(vec![
                Span::styled("Dur:", header_style()),
                Span::styled(format!(" {} ", enc.duration), value_style()),
                Span::raw("| "),
                Span::styled(format!("{}:", metric_label), header_style()),
                Span::styled(format!(" {} ", metric_val), value_style()),
                Span::raw("| "),
                Span::styled(format!("{}:", total_label), header_style()),
                Span::styled(format!(" {}", total_val), value_style()),
            ])
        } else if width >= 40 {
            Line::from(vec![
                Span::styled("Dur:", header_style()),
                Span::styled(format!(" {} ", enc.duration), value_style()),
                Span::styled(format!("{}:", metric_label), header_style()),
                Span::styled(format!(" {}", metric_val), value_style()),
            ])
        } else if width >= 28 {
            Line::from(vec![
                Span::styled(enc.duration.clone(), value_style()),
                Span::raw("  "),
                Span::styled(metric_val.to_string(), value_style()),
            ])
        } else {
            Line::from(vec![Span::styled(metric_val.to_string(), value_style())])
        }
    } else {
        Line::from(vec![Span::raw("Waiting for data...")])
    }
}

fn header_title_line(snapshot: &AppSnapshot, width: usize) -> Line<'static> {
    if let Some(enc) = &snapshot.encounter {
        let display_title = if enc.title.is_empty()
            || (enc.is_active && enc.title.eq_ignore_ascii_case("Encounter"))
        {
            enc.zone.clone()
        } else {
            enc.title.clone()
        };

        if width >= 40 {
            Line::from(vec![
                Span::styled("Encounter:", header_style()),
                Span::styled(format!(" {}  ", display_title), value_style()),
                Span::styled("Zone:", header_style()),
                Span::styled(format!(" {}", enc.zone), value_style()),
            ])
        } else if width >= 24 {
            Line::from(vec![
                Span::styled("Enc:", header_style()),
                Span::styled(format!(" {}  ", display_title), value_style()),
            ])
        } else {
            Line::from(vec![])
        }
    } else {
        Line::from(vec![])
    }
}
