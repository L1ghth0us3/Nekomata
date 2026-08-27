# Nekomata

<p align="center">
  <img src="assets/nekomata_logo_text.png" alt="Nekomata logo" width="200" />
</p>

**Nekomata** is a rust-based, dependency-light DPS meter for FFXIV that connects to the IINACT plugin over OverlayPlugin's WebSocket API and renders a kagerou-style table using ratatui.

## Features
- **Live combat data** displayed directly in your terminal with real-time updates
- **Dual view modes**: Swap between DPS and Heal modes with a single keypress
- **Encounter history**: Saves encounters in a sorted history list with a dedicated history panel
- **History views**: Swap between encounter history and dungeon-run history (`Tab`), and between DPS and Heal inside a record (`m`)
- **Visual decorations**: Cycle through three decoration styles (cycle with `d`):
  - `Decor: underline` — thin role-colored bar directly under each entry (two-line rows)
  - `Decor: background` — role-colored background meter behind each entry (one-line rows)
  - `Decor: none` — no extra decoration (compact one-line rows)
- **Themes**: Switch among bundled palettes in settings, or drop extra `.theme` files next to the binary
- **Limit Break display**: Show LB caster and damage as a dedicated panel, as a DPS table row, or hide it
- **Settings management**: Persistent configuration through config file and/or TUI settings pane (including nested **History Settings** for recording on/off, retention limits, backups, and archive browsing)
- **Idle mode**: Configurable idle detection with overlay toggle to peek at last encounter
- **Dungeon Mode**: Aggregate encounters into single dungeon runs while preserving individual encounter details
- **Modular architecture**: Clean, maintainable codebase with separated concerns

## Install

### Prebuilt binaries (v0.5+)

Download the archive for your OS from the [GitHub Releases](https://github.com/L1ghth0us3/Nekomata/releases) page:

- Linux x86_64 (glibc): `nekomata-v0.5-linux-x86_64.tar.gz`
- Windows x86_64: `nekomata-v0.5-windows-x86_64.zip`

Extract the archive and **keep the `themes/` folder next to the binary**. Themes are loaded from `<executable directory>/themes/`; if that folder is missing, Nekomata falls back to the built-in Synth Wave palette.

```bash
# Linux
tar -xzf nekomata-v0.5-linux-x86_64.tar.gz
cd nekomata-v0.5-linux-x86_64
./nekomata
```

```powershell
# Windows (PowerShell), after extracting the zip
.\nekomata.exe
```

On Windows, a modern terminal (Windows Terminal) is recommended.

## Dungeon Mode

Dungeon mode was created to address the limitations of other DPS interfaces which have overly simple encounter logic. This toggleable mode allows you to aggregate encounters into one single dungeon run while keeping detailed information on every separate encounter.

**How it works:**
- When enabled, encounters are automatically grouped by zone (defined in `dungeon-catalog.json`)
- All encounters within the same zone are saved under the same dungeon run
- When you enter a new zone, a new dungeon run begins automatically
- Use `Shift-D` to manually cut off a dungeon run and save it
- The history view includes a special "dungeon view" to browse aggregated runs
- Individual encounters within each dungeon run remain accessible for detailed analysis

## Prerequisites
- Rust 1.74+ (stable) recommended if you're building from source
- IINACT running locally (or reachable over your network)
  - Default WebSocket endpoint: `ws://127.0.0.1:10501/ws`

## Build from source & Run
```bash
# From the repo root (themes/ must be present here, or next to the built binary)
cargo run --release
# Write logs to the default config directory (~/.config/nekomata/debug.log)
cargo run --release -- --debug
# Or choose a custom log file path
cargo run --release -- --debug ./logs/nekomata-debug.log
```
The app will connect automatically to `ws://127.0.0.1:10501/ws` and begin rendering as soon as events arrive.

### Debug logging
- Pass `--debug` to enable file logging at startup. Without it, the TUI stays silent (no stdout/stderr noise).
- Supplying `--debug` with no value writes all tracing output (info/debug/warn/error) to `~/.config/nekomata/debug.log` on Unix-like systems or the equivalent config directory on Windows.
- Provide a path after `--debug` (e.g., `--debug ./logs/nekomata.log`) to log elsewhere; parent directories are created automatically if needed.

## Controls
- `q` or `Esc` — quit (or close settings / history)
- `d` — cycle decorations (underline → background → none)
- `m` — toggle table mode (DPS ↔ HEAL); in history, toggles DPS/Heal for the open record
- `s` — toggle the settings pane
- `h` — open/close the encounter history panel
- `i` — when idle mode is active, toggle the idle overlay on/off to peek at the last encounter
- `Shift-D` — when dungeon mode is active, cut off a dungeon run and save it
- `Tab` / `t` — in history, switch between encounter history and dungeon-run history
- `↑/↓` — move the selection inside the settings pane or history lists
- `←/→` — adjust the selected setting (idle timeout, decoration, mode, dungeon mode, limit break display, theme, role colors)

## Themes

Nekomata loads every `*.theme` file from `<executable directory>/themes/` at startup. Bundled presets:

- Synth Wave (default)
- Abyss Protocol
- Catppuccin Latte
- Catppuccin Mocha
- Gruvbox Retro
- Tokyo Night

Pick a theme from the settings pane (`s`). Turn **Change role specific colors** off to keep the classic xterm-256 tank/healer/DPS meter colors while still using the theme for text and accents.

A `.theme` file is TOML with `[meta]`, `[palette]`, `[roles]`, and optional `[jobs]` hex colors (`#RRGGBB`). Copy an existing preset as a starting point.

## Limit Break display

Limit Break casts are parsed from ability log lines. In settings, **Limit break display** can be:

- `PanelAlways` — a small panel under the table (name + damage)
- `TableRow` — a Limit Break row in the DPS table, sorted by ENCDPS
- `Off` — hidden

## Technical Notes & Behavior

### Data Processing
- **Party-only filtering**: Rows are filtered to common job codes (PLD/WAR/DRK/GNB, WHM/SCH/AST/SGE, MNK/DRG/NIN/SAM/RPR/VPR, BRD/MCH/DNC, BLM/SMN/RDM/PCT, BLU) plus pre-jobs (GLD/PGL/MRD/LNC/ARC/CNJ/THM/ROG)
- **Numeric normalization**: Numeric fields arrive as strings; commas/percent signs are stripped before parsing for sorting/ratios. Damage share is computed from per-combatant damage over encounter total
- **Encounter naming**: While a fight is active, some servers report generic names (e.g., "Encounter"); the header falls back to Zone until a final name is available

### UI & Styling
- **Terminal transparency**: Widgets avoid setting a background color so your terminal theme (blur/transparency) stays visible. The header separator uses a subtle gray; background meters intentionally set a background for the meter fill only
- **Responsive layout**: Table columns adapt to terminal width, with breakpoints that hide less critical columns on narrow displays

### Configuration & Persistence
- **Config location**: Settings are written to `~/.config/nekomata/nekomata.config` on Linux/macOS (or `%APPDATA%\nekomata\nekomata.config` on Windows)
- **Environment variables**: Set `NEKOMATA_CONFIG_DIR` to override the config directory, or `NEKOMATA_DUNGEON_CATALOG` to specify a custom dungeon catalog path (the catalog is otherwise embedded in the binary)
- **History storage**: Encounter history is stored in a sled-backed database at `~/.config/nekomata/history/encounters.sled` (or equivalent in your config directory). Manual backups are copied to `history/archives/<name>/`. Disabling history in History Settings stops recording without deleting existing data.

### History Panel
- Press `h` to switch into the history view
- Use `Tab` / `t` to swap between encounter history and dungeon-run history
- Use `↑/↓` or mouse scroll to pick a date
- Hit `Enter`/click to drill into the encounters (or dungeon runs) list
- Press `Enter` again for per-encounter details
- Use `←`/`Backspace` to step back
- Date and encounter lists load from lightweight indexes first, with overlay indicators while data hydrates
- Encounter detail fetches the full frame-by-frame record on demand

### Idle Mode
- When the app is idle, you'll see the idle window by default (including while disconnected, after the configured timeout)
- Press `i` to hide/show the idle overlay without leaving idle mode
- This allows you to review the most recent encounter quickly

## Troubleshooting
- Confirm IINACT is running and the endpoint is reachable. The default is `ws://127.0.0.1:10501/ws`.
- History or live table is empty? Only party and combat jobs are shown; pets are filtered out. Limit Break can be shown via the settings pane.
- Colors look like the default Synth Wave palette even after downloading a release? Make sure the `themes/` folder is still next to the executable.

## Roadmap

Short-term plans:
- Toggle for background opacity

For a complete list of changes, see [CHANGELOG.md](CHANGELOG.md).

## License
This project does not currently declare a license. Ask before redistributing.
