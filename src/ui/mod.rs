use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::history::util::{parse_duration_secs, parse_number};
use crate::model::{AppSnapshot, LimitBreakMode, LimitBreakSummary, ViewMode};
use crate::{ui_history, ui_idle};

mod header;
mod lb;
pub(crate) use lb::LB_PANEL_HEIGHT;
mod settings;
mod status;
mod table;
pub(crate) use table::{draw_with_context as draw_table_with_context, TableRenderContext};

pub fn draw(f: &mut Frame, snapshot: &AppSnapshot) {
    if snapshot.history.visible {
        ui_history::draw_history(f, snapshot);
        return;
    }

    let show_panel = snapshot.settings.limit_break_mode == LimitBreakMode::Panel;
    let show_table_row = snapshot.settings.limit_break_mode == LimitBreakMode::TableRow;
    let constraints = if show_panel {
        vec![
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(LB_PANEL_HEIGHT),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Length(3), Constraint::Min(4), Constraint::Length(1)]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.size());

    header::draw(f, chunks[0], snapshot);

    let mut snap_for_table;
    let snap_ref: &AppSnapshot = if snapshot.is_idle && snapshot.show_idle_overlay {
        ui_idle::draw_idle(f, chunks[1], snapshot);
        snapshot
    } else {
        snap_for_table = snapshot.clone();
        if show_table_row && snap_for_table.mode == ViewMode::Dps {
            if let Some(lb) = snap_for_table.lb_summary.as_ref() {
                snap_for_table.rows = build_lb_table_rows(snapshot, lb);
            }
        }
        &snap_for_table
    };

    // If we didn't draw idle overlay, render the table (possibly with LB injected).
    if !(snapshot.is_idle && snapshot.show_idle_overlay) {
        table::draw(f, chunks[1], snap_ref);
    }

    let status_idx = if show_panel {
        let placeholder = LimitBreakSummary {
            user: "—".to_string(),
            damage: 0,
        };
        let lb_ref = snapshot.lb_summary.as_ref().unwrap_or(&placeholder);
        lb::draw(f, chunks[2], lb_ref);
        3
    } else {
        2
    };

    if let Some(error) = snapshot.error.as_ref() {
        status::draw_error(f, chunks[status_idx], error);
    } else {
        status::draw(f, chunks[status_idx], snapshot);
    }

    if snapshot.show_settings {
        settings::draw(f, snapshot);
    }
}

fn format_number(value: f64) -> String {
    if value.abs() >= 1000.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn build_lb_table_rows(snapshot: &AppSnapshot, lb: &LimitBreakSummary) -> Vec<crate::model::CombatantRow> {
    use std::cmp::Ordering;
    let mut rows = snapshot.rows.clone();

    let total_damage = snapshot
        .encounter
        .as_ref()
        .map(|e| parse_number(&e.damage))
        .unwrap_or(0.0);

    let damage = lb.damage as f64;
    let share = if total_damage > 0.0 {
        (damage / total_damage).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let duration_secs = snapshot
        .encounter
        .as_ref()
        .and_then(|e| parse_duration_secs(&e.duration))
        .filter(|secs| *secs > 0)
        .map(|secs| secs as f64)
        .unwrap_or(1.0);
    let encdps = damage / duration_secs;

    let lb_row = crate::model::CombatantRow {
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
    };

    rows.push(lb_row);
    rows.sort_by(|a, b| {
        b.encdps
            .partial_cmp(&a.encdps)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}
