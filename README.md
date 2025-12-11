# Nekomata

<p align="center">
  <img src="nekomata_logo_text.png" alt="Nekomata logo" width="200" />
</p>

**Nekomata** is a rust-based, dependency-light DPS meter for FFXIV that connects to the IINACT plugin over OverlayPlugin's WebSocket API and renders a kagerou-style table using ratatui.

> **Note:** Nekomata v0.3.0 represents a complete rebrand from the previous project name, with major architectural improvements, enhanced history system, and new dungeon mode features. See [CHANGELOG.md](CHANGELOG.md) for details.

## Features
- **Live combat data** displayed directly in your terminal with real-time updates
- **Dual view modes**: Swap between DPS and Heal modes with a single keypress
- **Encounter history**: Saves encounters in a sorted history list with a dedicated history panel
- **History views**: Swap between DPS and Heal view in history panel
- **Visual decorations**: Cycle through three decoration styles (cycle with `d`):
  - `Decor: underline` — thin role-colored bar directly under each entry (two-line rows)
  - `Decor: background` — role-colored background meter behind each entry (one-line rows)
  - `Decor: none` — no extra decoration (compact one-line rows)
- **Settings management**: Persistent configuration through config file and/or TUI settings pane
- **Idle mode**: Configurable idle detection with overlay toggle to peek at last encounter
- **Dungeon Mode**: Aggregate encounters into single dungeon runs while preserving individual encounter details
- **Modular architecture**: Clean, maintainable codebase with separated concerns

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
# From the repo root
cargo run
# Write logs to the default config directory (~/.config/nekomata/debug.log)
cargo run -- --debug
# Or choose a custom log file path
cargo run -- --debug ./logs/nekomata-debug.log
```
The app will connect automatically to `ws://127.0.0.1:10501/ws` and begin rendering as soon as events arrive.

### Debug logging
- Pass `--debug` to enable file logging at startup. Without it, the TUI stays silent (no stdout/stderr noise).
- Supplying `--debug` with no value writes all tracing output (info/debug/warn/error) to `~/.config/nekomata/debug.log` on Unix-like systems or the equivalent config directory on Windows.
- Provide a path after `--debug` (e.g., `--debug ./logs/nekomata.log`) to log elsewhere; parent directories are created automatically if needed.

## Controls
- `q` or `Esc` — quit
- `d` — cycle decorations (underline → background → none)
- `m` — toggle table mode (DPS ↔ HEAL)
- `s` — toggle the settings pane
- `h` — open/close the encounter history panel
- `i` — when idle mode is active, toggle the idle overlay on/off to peek at the last encounter
- `Shift-D` — when dungeon mode is active, cut off a dungeon run and save it
- `↑/↓` — move the selection inside the settings pane
- `←/→` — adjust the selected setting (idle timeout, default decoration, default mode)

## Technical Notes & Behavior

### Data Processing
- **Party-only filtering**: Rows are filtered to common job codes (PLD/WAR/DRK/GNB, WHM/SCH/AST/SGE, MNK/DRG/NIN/SAM/RPR/VPR, BRD/MCH/DNC, BLM/SMN/RDM/PCT, BLU)
- **Numeric normalization**: Numeric fields arrive as strings; commas/percent signs are stripped before parsing for sorting/ratios. Damage share is computed from per-combatant damage over encounter total
- **Encounter naming**: While a fight is active, some servers report generic names (e.g., "Encounter"); the header falls back to Zone until a final name is available

### UI & Styling
- **Terminal transparency**: Widgets avoid setting a background color so your terminal theme (blur/transparency) stays visible. The header separator uses a subtle gray; background meters intentionally set a background for the meter fill only
- **Responsive layout**: Table columns adapt to terminal width, with breakpoints that hide less critical columns on narrow displays

### Configuration & Persistence
- **Config location**: Settings are written to `~/.config/nekomata/nekomata.config` on Linux/macOS (or `%APPDATA%\nekomata\nekomata.config` on Windows)
- **Environment variables**: Set `NEKOMATA_CONFIG_DIR` to override the config directory, or `NEKOMATA_DUNGEON_CATALOG` to specify a custom dungeon catalog path
- **History storage**: Encounter history is stored in a sled-backed database at `~/.config/nekomata/history/encounters.sled` (or equivalent in your config directory)

### History Panel
- Press `h` to switch into the history view
- Use `↑/↓` or mouse scroll to pick a date
- Hit `Enter`/click to drill into the encounters list
- Press `Enter` again for per-encounter details
- Use `←`/`Backspace` to step back
- Date and encounter lists load from lightweight indexes first, with overlay indicators while data hydrates
- Encounter detail fetches the full frame-by-frame record on demand

### Idle Mode
- When the app is idle, you'll see the idle window by default
- Press `i` to hide/show the idle overlay without leaving idle mode
- This allows you to review the most recent encounter quickly

## Troubleshooting
- Confirm IINACT is running and the endpoint is reachable. The default is `ws://127.0.0.1:10501/ws`.
- History or live table is empty? Only party and combat jobs are shown; pets/limit break lines are filtered out. (for now)

## Roadmap

Short-term plans:
- Dedicated Limit Break window (who/when/how much/what level)
- Theme presets (purple/cyberpunk, monochrome, gray meters)
- Toggle for background opacity

For a complete list of changes, see [CHANGELOG.md](CHANGELOG.md).

## License
This project does not currently declare a license. Ask before redistributing.
