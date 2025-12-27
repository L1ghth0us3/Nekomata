use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::model::{AppSnapshot, IdleScene};
use crate::theme::{header_style, title_style, value_style, TEXT};

/// Default order new idle widgets should rotate through once rotation logic lands.
#[allow(dead_code)]
pub const DEFAULT_ROTATION: [IdleScene; 4] = [
    IdleScene::TopCritChain,
    IdleScene::TipOfTheDay,
    IdleScene::AsciiArt,
    IdleScene::AchievementTicker,
];

pub fn draw_idle(f: &mut Frame, area: Rect, snapshot: &AppSnapshot) {
    f.render_widget(Clear, area);

    let [header_area, body_area] = split_idle(area);
    render_header(f, header_area, snapshot);
    render_scene(f, body_area, snapshot);
}

fn split_idle(area: Rect) -> [Rect; 2] {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    [layout[0], layout[1]]
}

fn render_header(f: &mut Frame, area: Rect, snapshot: &AppSnapshot) {
    let title = Line::from(vec![
        Span::styled("Idle mode", title_style()),
        Span::raw("  •  "),
        Span::styled(snapshot.idle_scene.label(), header_style()),
    ]);

    let description = Line::from(vec![Span::styled(
        snapshot.idle_scene.description(),
        Style::default().fg(TEXT).add_modifier(Modifier::DIM),
    )]);

    let block = Block::default().borders(Borders::NONE);
    let mut lines = vec![title, description];
    if snapshot.idle_scene == IdleScene::Status {
        lines.push(Line::from(vec![Span::styled(
            "press 'i' to toggle idle window",
            Style::default().fg(TEXT).add_modifier(Modifier::DIM),
        )]));
    }

    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(widget, area);
}

fn render_scene(f: &mut Frame, area: Rect, snapshot: &AppSnapshot) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "Coming soon",
            header_style(),
        )]))
        .borders(Borders::ALL);

    let lines = scene_lines(snapshot);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn scene_lines(snapshot: &AppSnapshot) -> Vec<Line<'static>> {
    match snapshot.idle_scene {
        IdleScene::Status => status_lines(snapshot),
        IdleScene::TopCritChain => placeholder(
            "Top crit chain",
            "This panel will highlight the largest crit sequences across the party.",
        ),
        IdleScene::AsciiArt => placeholder(
            "ASCII art rotation",
            "Drop .txt art here and the idle loop will cycle through it.",
        ),
        IdleScene::TipOfTheDay => placeholder(
            "Tip of the day",
            "Surface encounter prep, rotation tips, or community callouts.",
        ),
        IdleScene::AchievementTicker => placeholder(
            "Achievement ticker",
            "Showcase recent clears, parses, and personal bests.",
        ),
    }
}

fn status_lines(snapshot: &AppSnapshot) -> Vec<Line<'static>> {
    let connection = if snapshot.connected {
        if snapshot.is_idle {
            "Connected (idle)"
        } else {
            "Connected"
        }
    } else {
        if snapshot.is_idle {
            "Disconnected (idle)"
        } else {
            "Disconnected"
        }
    };

    let encounter_label = snapshot
        .encounter
        .as_ref()
        .map(|enc| {
            if enc.title.is_empty() {
                enc.zone.clone()
            } else {
                enc.title.clone()
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "No active encounter".to_string());

    vec![
        Line::from(vec![Span::styled(connection, value_style())]),
        Line::from(vec![Span::styled(encounter_label, value_style())]),
    ]
}

fn placeholder(title: &str, caption: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(title.to_string(), value_style())]),
        Line::from(vec![Span::styled(caption.to_string(), header_style())]),
        Line::from(vec![Span::styled(
            "Rotate scenes via DEFAULT_ROTATION or update AppState::idle_scene.",
            Style::default().fg(TEXT).add_modifier(Modifier::DIM),
        )]),
    ]
}
