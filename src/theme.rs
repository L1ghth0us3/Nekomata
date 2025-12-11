use ratatui::style::{Color, Style};

// Dark purple / cyberpunk palette (foreground-only to preserve terminal background)
pub const ACCENT: Color = Color::Rgb(200, 60, 255); // neon purple
pub const ACCENT_2: Color = Color::Rgb(0, 255, 200); // neon cyan-green
pub const TEXT: Color = Color::Rgb(220, 210, 230);
pub const STATUS_IDLE: Color = Color::Rgb(205, 102, 0); // dark orange
pub const STATUS_DISCONNECTED: Color = Color::Rgb(220, 60, 60); // bright red

// Simple job color suggestions tuned toward purple/cyberpunk vibe
pub fn job_color(job: &str) -> Color {
    match job {
        // Tanks
        "PLD" => Color::Rgb(180, 160, 255),
        "WAR" => Color::Rgb(255, 120, 120),
        "DRK" => Color::Rgb(150, 60, 200),
        "GNB" => Color::Rgb(200, 120, 255),
        // Healers
        "WHM" => Color::Rgb(200, 220, 255),
        "SCH" => Color::Rgb(120, 200, 255),
        "AST" => Color::Rgb(255, 180, 255),
        "SGE" => Color::Rgb(120, 255, 230),
        // Melee
        "MNK" => Color::Rgb(255, 200, 140),
        "DRG" => Color::Rgb(140, 160, 255),
        "NIN" => Color::Rgb(255, 100, 200),
        "SAM" => Color::Rgb(255, 120, 160),
        "RPR" => Color::Rgb(180, 80, 180),
        "VPR" => Color::Rgb(220, 120, 255),
        // Ranged phys
        "BRD" => Color::Rgb(255, 200, 255),
        "MCH" => Color::Rgb(160, 255, 220),
        "DNC" => Color::Rgb(255, 160, 220),
        // Casters
        "BLM" => Color::Rgb(120, 120, 255),
        "SMN" => Color::Rgb(120, 255, 160),
        "RDM" => Color::Rgb(255, 160, 200),
        "PCT" => Color::Rgb(180, 220, 255),
        // Limited
        "BLU" => Color::Rgb(140, 200, 255),
        // Pre-Jobs
        "GLD" => Color::Rgb(255, 200, 140),
        "PGL" => Color::Rgb(140, 160, 255),
        "MRD" => Color::Rgb(255, 100, 200),
        "LNC" => Color::Rgb(255, 120, 160),
        "ARC" => Color::Rgb(180, 80, 180),
        "CNJ" => Color::Rgb(120, 255, 230),
        "THM" => Color::Rgb(220, 120, 255),
        "ROG" => Color::Rgb(120, 200, 255),
        _ => ACCENT,
    }
}

pub fn header_style() -> Style {
    Style::default().fg(TEXT)
}
pub fn title_style() -> Style {
    Style::default().fg(ACCENT)
}
pub fn value_style() -> Style {
    Style::default().fg(ACCENT_2)
}

// Role-based color for DPS bars (xterm 256-indexed colors)
// Tanks → blue(75), Healers → green(41), DPS → red(124)
pub fn role_bar_color(job: &str) -> Color {
    match job {
        // Tanks
        "PLD" | "WAR" | "DRK" | "GNB" | "GLA" | "MRD" => Color::Indexed(75),
        // Healers
        "WHM" | "SCH" | "AST" | "SGE" | "CNJ" => Color::Indexed(41),
        // Everything else treated as DPS
        _ => Color::Indexed(124),
    }
}

// Gradient helpers removed; we use solid role colors for bars.
