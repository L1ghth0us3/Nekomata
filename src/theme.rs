use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, atomic::Ordering, OnceLock, RwLock, RwLockReadGuard};

use ratatui::style::{Color, Style};
use serde::Deserialize;

/// Theme describes the full color palette used by the TUI.
/// For v0.5 this is effectively a struct wrapper around the existing
/// Synth Wave palette so we can later swap themes at runtime.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub accent: Color,
    pub accent_2: Color,
    pub text: Color,
    pub status_idle: Color,
    pub status_disconnected: Color,
    /// Base colors per role for table meters/underlines.
    pub tank_color: Color,
    pub healer_color: Color,
    pub dps_color: Color,
    /// Job-specific foreground colors.
    pub job_colors: HashMap<String, Color>,
}

impl Theme {
    /// Synth Wave theme matching the pre-v0.5 hardcoded palette.
    pub fn synth_wave() -> Self {
        let mut job_colors = HashMap::new();

        // Tanks
        job_colors.insert("PLD".to_string(), Color::Rgb(180, 160, 255));
        job_colors.insert("WAR".to_string(), Color::Rgb(255, 120, 120));
        job_colors.insert("DRK".to_string(), Color::Rgb(150, 60, 200));
        job_colors.insert("GNB".to_string(), Color::Rgb(200, 120, 255));
        // Healers
        job_colors.insert("WHM".to_string(), Color::Rgb(200, 220, 255));
        job_colors.insert("SCH".to_string(), Color::Rgb(120, 200, 255));
        job_colors.insert("AST".to_string(), Color::Rgb(255, 180, 255));
        job_colors.insert("SGE".to_string(), Color::Rgb(120, 255, 230));
        // Melee
        job_colors.insert("MNK".to_string(), Color::Rgb(255, 200, 140));
        job_colors.insert("DRG".to_string(), Color::Rgb(140, 160, 255));
        job_colors.insert("NIN".to_string(), Color::Rgb(255, 100, 200));
        job_colors.insert("SAM".to_string(), Color::Rgb(255, 120, 160));
        job_colors.insert("RPR".to_string(), Color::Rgb(180, 80, 180));
        job_colors.insert("VPR".to_string(), Color::Rgb(220, 120, 255));
        // Ranged phys
        job_colors.insert("BRD".to_string(), Color::Rgb(255, 200, 255));
        job_colors.insert("MCH".to_string(), Color::Rgb(160, 255, 220));
        job_colors.insert("DNC".to_string(), Color::Rgb(255, 160, 220));
        // Casters
        job_colors.insert("BLM".to_string(), Color::Rgb(120, 120, 255));
        job_colors.insert("SMN".to_string(), Color::Rgb(120, 255, 160));
        job_colors.insert("RDM".to_string(), Color::Rgb(255, 160, 200));
        job_colors.insert("PCT".to_string(), Color::Rgb(180, 220, 255));
        // Limited
        job_colors.insert("BLU".to_string(), Color::Rgb(140, 200, 255));
        // Pre-Jobs
        job_colors.insert("GLD".to_string(), Color::Rgb(255, 200, 140));
        job_colors.insert("PGL".to_string(), Color::Rgb(140, 160, 255));
        job_colors.insert("MRD".to_string(), Color::Rgb(255, 100, 200));
        job_colors.insert("LNC".to_string(), Color::Rgb(255, 120, 160));
        job_colors.insert("ARC".to_string(), Color::Rgb(180, 80, 180));
        job_colors.insert("CNJ".to_string(), Color::Rgb(120, 255, 230));
        job_colors.insert("THM".to_string(), Color::Rgb(220, 120, 255));
        job_colors.insert("ROG".to_string(), Color::Rgb(120, 200, 255));

        Theme {
            name: "Synth Wave".to_string(),
            accent: Color::Rgb(200, 60, 255),          // neon purple
            accent_2: Color::Rgb(0, 255, 200),         // neon cyan-green
            text: Color::Rgb(220, 210, 230),
            status_idle: Color::Rgb(205, 102, 0),      // dark orange
            status_disconnected: Color::Rgb(220, 60, 60), // bright red
            // Role meter colors currently match the legacy xterm indices.
            tank_color: Color::Indexed(75),
            healer_color: Color::Indexed(41),
            dps_color: Color::Indexed(124),
            job_colors,
        }
    }

    pub fn job_color(&self, job: &str) -> Color {
        self.job_colors
            .get(job)
            .copied()
            .unwrap_or(self.accent)
    }

    pub fn role_bar_color(&self, job: &str) -> Color {
        match job {
            // Tanks
            "PLD" | "WAR" | "DRK" | "GNB" | "GLD" | "MRD" => self.tank_color,
            // Healers
            "WHM" | "SCH" | "AST" | "SGE" | "CNJ" => self.healer_color,
            // Everything else treated as DPS
            _ => self.dps_color,
        }
    }
}

// Global active theme handle. For v0.5 this is initialized to Synth Wave
// and will later be updated when loading .theme files and applying settings.
static ACTIVE_THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
static THEME_REGISTRY: OnceLock<RwLock<ThemeRegistry>> = OnceLock::new();
static ROLE_THEME_ENABLED: AtomicBool = AtomicBool::new(true);

fn active_theme_lock() -> &'static RwLock<Theme> {
    ACTIVE_THEME.get_or_init(|| RwLock::new(Theme::synth_wave()))
}

pub fn active_theme() -> RwLockReadGuard<'static, Theme> {
    active_theme_lock()
        .read()
        .expect("active theme lock poisoned")
}

pub fn set_active_theme(theme: Theme) {
    let lock = active_theme_lock();
    if let Ok(mut guard) = lock.write() {
        *guard = theme;
    }
}

pub fn header_style() -> Style {
    Style::default().fg(active_theme().text)
}

pub fn title_style() -> Style {
    Style::default().fg(active_theme().accent)
}

pub fn value_style() -> Style {
    Style::default().fg(active_theme().accent_2)
}

pub fn status_idle_color() -> Color {
    active_theme().status_idle
}

pub fn status_disconnected_color() -> Color {
    active_theme().status_disconnected
}

pub fn job_color(job: &str) -> Color {
    active_theme().job_color(job)
}

pub fn role_bar_color(job: &str) -> Color {
    if !ROLE_THEME_ENABLED.load(Ordering::Relaxed) {
        // Legacy role colors: Tanks blue(75), Healers green(41), DPS red(124)
        return match job {
            "PLD" | "WAR" | "DRK" | "GNB" | "GLD" | "MRD" => Color::Indexed(75),
            "WHM" | "SCH" | "AST" | "SGE" | "CNJ" => Color::Indexed(41),
            _ => Color::Indexed(124),
        };
    }
    active_theme().role_bar_color(job)
}

pub fn text_color() -> Color {
    active_theme().text
}

pub fn set_role_theme_enabled(enabled: bool) {
    ROLE_THEME_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn accent_color() -> Color {
    active_theme().accent
}

pub fn accent2_color() -> Color {
    active_theme().accent_2
}

// Gradient helpers removed; we use solid role colors for bars.

/// Public description of a theme option presented in the UI.
#[derive(Clone, Debug)]
pub struct ThemeDescriptor {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct ThemeRegistry {
    themes: Vec<(ThemeDescriptor, Theme)>,
}

impl ThemeRegistry {
    fn synth_wave_fallback() -> Self {
        let theme = Theme::synth_wave();
        let descriptor = ThemeDescriptor {
            id: "SynthWave".to_string(),
            name: theme.name.clone(),
        };
        ThemeRegistry {
            themes: vec![(descriptor, theme)],
        }
    }

    fn from_filesystem(dir: &Path) -> Self {
        let mut themes: Vec<(ThemeDescriptor, Theme)> = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("theme"))
                    != Some(true)
                {
                    continue;
                }

                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match load_theme_file(stem, &path) {
                        Ok(theme) => {
                            let descriptor = ThemeDescriptor {
                                id: stem.to_string(),
                                name: theme.name.clone(),
                            };
                            themes.push((descriptor, theme));
                        }
                        Err(err) => {
                            eprintln!("Failed to load theme from {}: {err}", path.display());
                        }
                    }
                }
            }
        }

        if themes.is_empty() {
            ThemeRegistry::synth_wave_fallback()
        } else {
            themes.sort_by(|a, b| a.0.id.cmp(&b.0.id));
            ThemeRegistry { themes }
        }
    }

    pub fn descriptors(&self) -> Vec<ThemeDescriptor> {
        self.themes.iter().map(|(d, _)| d.clone()).collect()
    }

    pub fn get_theme(&self, id: &str) -> Option<&Theme> {
        self.themes
            .iter()
            .find(|(d, _)| d.id.eq_ignore_ascii_case(id))
            .map(|(_, t)| t)
    }

    pub fn default_id(&self) -> String {
        // Prefer SynthWave.theme when present.
        if let Some((d, _)) = self
            .themes
            .iter()
            .find(|(d, _)| d.id.eq_ignore_ascii_case("SynthWave"))
        {
            return d.id.clone();
        }
        self.themes
            .first()
            .map(|(d, _)| d.id.clone())
            .unwrap_or_else(|| "SynthWave".to_string())
    }
}

fn theme_registry_lock() -> &'static RwLock<ThemeRegistry> {
    THEME_REGISTRY.get_or_init(|| {
        let dir = default_themes_dir();
        RwLock::new(ThemeRegistry::from_filesystem(&dir))
    })
}

pub fn theme_registry() -> RwLockReadGuard<'static, ThemeRegistry> {
    theme_registry_lock()
        .read()
        .expect("theme registry lock poisoned")
}

/// Apply a theme by id, falling back to the registry default when the id is
/// empty or unknown. Returns the effective id that was applied.
pub fn apply_theme_by_id(id: &str) -> String {
    let registry = theme_registry();
    let effective_id = if let Some((descriptor, theme)) = registry
        .themes
        .iter()
        .find(|(d, _)| d.id.eq_ignore_ascii_case(id))
    {
        set_active_theme(theme.clone());
        descriptor.id.clone()
    } else {
        let fallback_id = registry.default_id();
        if let Some(theme) = registry.get_theme(&fallback_id) {
            set_active_theme(theme.clone());
        } else {
            set_active_theme(Theme::synth_wave());
        }
        fallback_id
    };
    effective_id
}

fn default_themes_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("themes");
        }
    }
    PathBuf::from("themes")
}

#[derive(Deserialize)]
struct FileTheme {
    #[serde(default)]
    meta: FileThemeMeta,
    palette: FileThemePalette,
    roles: FileThemeRoles,
    #[serde(default)]
    jobs: HashMap<String, String>,
}

#[derive(Default, Deserialize)]
struct FileThemeMeta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct FileThemePalette {
    accent: String,
    #[serde(default)]
    accent_2: Option<String>,
    text: String,
    status_idle: String,
    status_disconnected: String,
}

#[derive(Deserialize)]
struct FileThemeRoles {
    tank: String,
    healer: String,
    dps: String,
}

fn load_theme_file(id: &str, path: &Path) -> Result<Theme, String> {
    let data = fs::read(path)
        .map_err(|err| format!("failed to read theme file {}: {err}", path.display()))?;
    let text = String::from_utf8(data)
        .map_err(|err| format!("failed to read theme file {} as UTF-8: {err}", path.display()))?;
    let parsed: FileTheme = toml::from_str(&text)
        .map_err(|err| format!("failed to parse theme TOML {}: {err}", path.display()))?;

    let name = parsed
        .meta
        .name
        .clone()
        .unwrap_or_else(|| id.to_string());

    let accent = parse_hex_color(&parsed.palette.accent)
        .ok_or_else(|| format!("invalid accent color in {}", path.display()))?;
    let accent_2 = parsed
        .palette
        .accent_2
        .as_ref()
        .and_then(|s| parse_hex_color(s))
        .unwrap_or(accent);
    let text = parse_hex_color(&parsed.palette.text)
        .ok_or_else(|| format!("invalid text color in {}", path.display()))?;
    let status_idle = parse_hex_color(&parsed.palette.status_idle)
        .ok_or_else(|| format!("invalid status_idle color in {}", path.display()))?;
    let status_disconnected = parse_hex_color(&parsed.palette.status_disconnected)
        .ok_or_else(|| format!("invalid status_disconnected color in {}", path.display()))?;

    let tank_color = parse_hex_color(&parsed.roles.tank)
        .ok_or_else(|| format!("invalid tank role color in {}", path.display()))?;
    let healer_color = parse_hex_color(&parsed.roles.healer)
        .ok_or_else(|| format!("invalid healer role color in {}", path.display()))?;
    let dps_color = parse_hex_color(&parsed.roles.dps)
        .ok_or_else(|| format!("invalid dps role color in {}", path.display()))?;

    let mut job_colors: HashMap<String, Color> = HashMap::new();
    for (job, hex) in parsed.jobs.into_iter() {
        if let Some(color) = parse_hex_color(&hex) {
            job_colors.insert(job.to_uppercase(), color);
        }
    }

    Ok(Theme {
        name,
        accent,
        accent_2,
        text,
        status_idle,
        status_disconnected,
        tank_color,
        healer_color,
        dps_color,
        job_colors,
    })
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
