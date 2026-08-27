use std::cmp::Ordering;

use ratatui::layout::Alignment;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::history::util::{format_number, parse_duration_secs, parse_number};
use crate::model::{CombatantRow, EncounterSummary, LimitBreakSummary};
use crate::theme::{header_style, title_style, value_style};

/// Min vertical space for borders (2) + two text lines (2).
pub(crate) const LB_PANEL_HEIGHT: u16 = 3;

pub(crate) fn build_lb_table_row(
    lb: &LimitBreakSummary,
    encounter: &EncounterSummary,
) -> CombatantRow {
    let total_damage = parse_number(&encounter.damage);
    let damage = lb.damage as f64;
    let share = if total_damage > 0.0 {
        (damage / total_damage).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let duration_secs = parse_duration_secs(&encounter.duration)
        .filter(|secs| *secs > 0)
        .map(|secs| secs as f64)
        .unwrap_or(1.0);
    let encdps = damage / duration_secs;

    CombatantRow {
        name: "Limit Break".to_string(),
        job: "LB".to_string(),
        encdps,
        encdps_str: format_number(encdps),
        damage,
        damage_str: format_number(damage),
        share,
        share_str: format!("{:.1}%", share * 100.0),
        enchps: 0.0,
        enchps_str: "0".to_string(),
        healed: 0.0,
        healed_str: "0".to_string(),
        heal_share: 0.0,
        heal_share_str: "0.0%".to_string(),
        overheal_pct: "0".to_string(),
        crit: "0".to_string(),
        dh: "0".to_string(),
        deaths: "0".to_string(),
    }
}

pub(crate) fn inject_lb_table_row(
    rows: &mut Vec<CombatantRow>,
    lb: &LimitBreakSummary,
    encounter: &EncounterSummary,
    mode: crate::model::ViewMode,
) {
    if mode != crate::model::ViewMode::Dps {
        return;
    }
    rows.push(build_lb_table_row(lb, encounter));
    rows.sort_by(|a, b| {
        b.encdps
            .partial_cmp(&a.encdps)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
}

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
