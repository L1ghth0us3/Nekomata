use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::model::{AppSnapshot, LimitBreakMode, LimitBreakSummary, ViewMode};
use crate::{ui_history, ui_idle};

mod encounter_detail;
mod header;
mod lb;
pub(crate) use encounter_detail::{draw_encounter_record, EncounterDetailParams};
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
            if let (Some(lb), Some(enc)) = (
                snap_for_table.lb_summary.as_ref(),
                snap_for_table.encounter.as_ref(),
            ) {
                let mut rows = snap_for_table.rows.clone();
                lb::inject_lb_table_row(&mut rows, lb, enc, ViewMode::Dps);
                snap_for_table.rows = rows;
            }
        }
        &snap_for_table
    };

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
