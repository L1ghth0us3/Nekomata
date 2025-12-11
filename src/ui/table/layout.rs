use ratatui::layout::Constraint;
use ratatui::style::Style;
use ratatui::widgets::{Cell, Row};

use crate::model::{CombatantRow, ViewMode};
use crate::theme::{header_style, job_color};

pub(super) struct LayoutSpec {
    columns: Vec<ColumnSpec>,
    header_height: u16,
    column_spacing: u16,
}

impl LayoutSpec {
    pub(super) fn header_height(&self) -> u16 {
        self.header_height
    }

    pub(super) fn column_spacing(&self) -> u16 {
        self.column_spacing
    }

    pub(super) fn header_row(&self) -> Row<'static> {
        Row::new(self.columns.iter().map(ColumnSpec::header_cell))
            .style(header_style())
            .height(self.header_height)
    }

    pub(super) fn data_row(&self, row: &CombatantRow, row_height: u16) -> Row<'static> {
        Row::new(self.columns.iter().map(|col| col.data_cell(row))).height(row_height)
    }

    pub(super) fn widths(&self) -> Vec<Constraint> {
        self.columns.iter().map(|col| col.width).collect()
    }
}

impl LayoutSpec {
    fn new(columns: Vec<ColumnSpec>) -> Self {
        Self {
            columns,
            header_height: 2,
            column_spacing: 1,
        }
    }

    fn with_spacing(mut self, spacing: u16) -> Self {
        self.column_spacing = spacing;
        self
    }
}

pub(super) fn layout_for(mode: ViewMode, width: usize) -> LayoutSpec {
    let variant = TableVariant::from_width(width);
    layout_for_variant(mode, variant)
}

fn layout_for_variant(mode: ViewMode, variant: TableVariant) -> LayoutSpec {
    match (mode, variant) {
        (ViewMode::Dps, TableVariant::Full) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(34)),
            right_column("Share%", 7, Constraint::Length(7), value_share),
            right_column("ENCDPS", 10, Constraint::Length(10), value_encdps),
            right_column("Job", 5, Constraint::Length(5), value_job),
            right_column("Crit%", 8, Constraint::Length(8), value_crit),
            right_column("DH%", 8, Constraint::Length(8), value_dh),
            right_column("Deaths", 8, Constraint::Length(8), value_deaths),
        ]),
        (ViewMode::Heal, TableVariant::Full) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(34)),
            right_column("Heal%", 7, Constraint::Length(7), value_heal_share),
            right_column("ENCHPS", 10, Constraint::Length(10), value_enchps),
            right_column("Job", 5, Constraint::Length(5), value_job),
            right_column("Overheal%", 10, Constraint::Length(10), value_overheal),
            right_column("Deaths", 8, Constraint::Length(8), value_deaths),
        ]),
        (ViewMode::Dps, TableVariant::NoDeaths) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(38)),
            right_column("Share%", 7, Constraint::Length(7), value_share),
            right_column("ENCDPS", 9, Constraint::Length(9), value_encdps),
            right_column("Job", 5, Constraint::Length(5), value_job),
            right_column("Crit%", 6, Constraint::Length(6), value_crit),
            right_column("DH%", 6, Constraint::Length(6), value_dh),
        ]),
        (ViewMode::Heal, TableVariant::NoDeaths) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(44)),
            right_column("Heal%", 7, Constraint::Length(7), value_heal_share),
            right_column("ENCHPS", 9, Constraint::Length(9), value_enchps),
            right_column("Job", 5, Constraint::Length(5), value_job),
            right_column("Overheal%", 9, Constraint::Length(9), value_overheal),
        ]),
        (ViewMode::Dps, TableVariant::NoDhDeaths) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(54)),
            right_column("Share%", 7, Constraint::Length(7), value_share),
            right_column("ENCDPS", 9, Constraint::Length(9), value_encdps),
            right_column("Crit%", 6, Constraint::Length(6), value_crit),
        ]),
        (ViewMode::Heal, TableVariant::NoDhDeaths) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(58)),
            right_column("Heal%", 7, Constraint::Length(7), value_heal_share),
            right_column("ENCHPS", 9, Constraint::Length(9), value_enchps),
            right_column("Job", 5, Constraint::Length(5), value_job),
        ]),
        (ViewMode::Dps, TableVariant::Minimal) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(64)),
            right_column("Share%", 6, Constraint::Length(6), value_share),
            right_column("ENCDPS", 9, Constraint::Length(9), value_encdps),
        ]),
        (ViewMode::Heal, TableVariant::Minimal) => LayoutSpec::new(vec![
            name_column(Constraint::Percentage(64)),
            right_column("Heal%", 6, Constraint::Length(6), value_heal_share),
            right_column("ENCHPS", 9, Constraint::Length(9), value_enchps),
        ]),
        (ViewMode::Dps, TableVariant::NameOnly) => LayoutSpec::new(vec![left_column(
            "Name (Share%)",
            Constraint::Percentage(100),
            value_name_with_share,
            Some(name_style),
        )])
        .with_spacing(0),
        (ViewMode::Heal, TableVariant::NameOnly) => LayoutSpec::new(vec![left_column(
            "Name (Heal%)",
            Constraint::Percentage(100),
            value_name_with_heal_share,
            Some(name_style),
        )])
        .with_spacing(0),
    }
}

#[derive(Copy, Clone)]
enum TableVariant {
    Full,
    NoDeaths,
    NoDhDeaths,
    Minimal,
    NameOnly,
}

impl TableVariant {
    fn from_width(width: usize) -> Self {
        if width >= 90 {
            TableVariant::Full
        } else if width >= 72 {
            TableVariant::NoDeaths
        } else if width >= 58 {
            TableVariant::NoDhDeaths
        } else if width >= 44 {
            TableVariant::Minimal
        } else {
            TableVariant::NameOnly
        }
    }
}

enum Align {
    Left,
    Right { width: usize },
}

impl Align {
    fn format(&self, text: &str) -> String {
        match self {
            Align::Left => text.to_string(),
            Align::Right { width } => right_align(text, *width),
        }
    }
}

struct ColumnSpec {
    header: &'static str,
    align: Align,
    width: Constraint,
    value: fn(&CombatantRow) -> String,
    style: Option<fn(&CombatantRow) -> Style>,
}

impl ColumnSpec {
    fn header_cell(&self) -> Cell<'static> {
        Cell::from(self.align.format(self.header))
    }

    fn data_cell(&self, row: &CombatantRow) -> Cell<'static> {
        let text = (self.value)(row);
        let formatted = self.align.format(&text);
        let mut cell = Cell::from(formatted);
        if let Some(style_fn) = self.style {
            cell = cell.style(style_fn(row));
        }
        cell
    }
}

fn name_style(row: &CombatantRow) -> Style {
    Style::default().fg(job_color(&row.job))
}

fn name_column(width: Constraint) -> ColumnSpec {
    ColumnSpec {
        header: "Name",
        align: Align::Left,
        width,
        value: value_name,
        style: Some(name_style),
    }
}

fn right_column(
    header: &'static str,
    align_width: usize,
    width: Constraint,
    value: fn(&CombatantRow) -> String,
) -> ColumnSpec {
    ColumnSpec {
        header,
        align: Align::Right { width: align_width },
        width,
        value,
        style: None,
    }
}

fn left_column(
    header: &'static str,
    width: Constraint,
    value: fn(&CombatantRow) -> String,
    style: Option<fn(&CombatantRow) -> Style>,
) -> ColumnSpec {
    ColumnSpec {
        header,
        align: Align::Left,
        width,
        value,
        style,
    }
}

fn value_name(row: &CombatantRow) -> String {
    row.name.clone()
}

fn value_share(row: &CombatantRow) -> String {
    row.share_str.clone()
}

fn value_heal_share(row: &CombatantRow) -> String {
    row.heal_share_str.clone()
}

fn value_encdps(row: &CombatantRow) -> String {
    row.encdps_str.clone()
}

fn value_enchps(row: &CombatantRow) -> String {
    row.enchps_str.clone()
}

fn value_job(row: &CombatantRow) -> String {
    row.job.clone()
}

fn value_crit(row: &CombatantRow) -> String {
    row.crit.clone()
}

fn value_dh(row: &CombatantRow) -> String {
    row.dh.clone()
}

fn value_deaths(row: &CombatantRow) -> String {
    row.deaths.clone()
}

fn value_overheal(row: &CombatantRow) -> String {
    row.overheal_pct.clone()
}

fn value_name_with_share(row: &CombatantRow) -> String {
    format!("{}  [{}]", row.name, row.share_str)
}

fn value_name_with_heal_share(row: &CombatantRow) -> String {
    format!("{}  [{}]", row.name, row.heal_share_str)
}

fn right_align(text: &str, width: usize) -> String {
    let len = text.len();
    if len >= width {
        text.chars()
            .rev()
            .take(width)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    } else {
        format!("{:>width$}", text, width = width)
    }
}
