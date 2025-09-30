# iinact-tui

A rust-based, dependency‑light DPS meter for FFXIV based entirely in the terminal that connects to the IINACT Plugin and renders a kagerou‑style table using ratatui.

## Features
- Show live combat data directly in your terminal
- Swap between DPS and Heal modes
- Saves encounters in a sorted history list ...
- ... and displays them in a dedicated window!
- Swap between DPS and Heal view in history.
- Change the visual decorations with a simple hotkey (more to come...)
- Decorations currently implemented (cycle with `d`):
  - `Decor: underline` — thin role-colored bar directly under each entry (two-line rows).
  - `Decor: background` — role-colored background meter behind each entry (one-line rows).
  - `Decor: none` — no extra decoration (compact one-line rows).
- Simple settings management through .config file and/or TUI
- Configurable 'Idle Mode' (more to come...)
- Dungeon Mode!

## Dungeon Mode
Dungeon mode was made out of frustration from other DPS interfaces which have very simple logic regarding encounters. This toggleable mode allows you to aggregate encounters into one single dungeon run, while keeping information on every separate encounter. Whenever you enter a new Zone (defined in dungeon-catalog.json), if toggled on, every encounter will be saved under the same umbrella dungeon Zone until the zone changes again. This is also reflected in the history view with a special "dungeon view".

## Prerequisites
- Rust 1.74+ (stable) recommended if you're building from source
- IINACT running locally (or reachable over your network)
  - Default WebSocket endpoint: `ws://127.0.0.1:10501/ws`

## Build from source & Run
```bash
# From the repo root
cargo run
# Write logs to the default config directory (~/.config/iinact-tui/debug.log)
cargo run -- --debug
# Or choose a custom log file path
cargo run -- --debug ./logs/iinact-debug.log
```
The app will connect automatically to `ws://127.0.0.1:10501/ws` and begin rendering as soon as events arrive.

### Debug logging
- Pass `--debug` to enable file logging at startup. Without it, the TUI stays silent (no stdout/stderr noise).
- Supplying `--debug` with no value writes all tracing output (info/debug/warn/error) to `~/.config/iinact-tui/debug.log` on Unix-like systems or the equivalent config directory on Windows.
- Provide a path after `--debug` (e.g., `--debug ./logs/iinact.log`) to log elsewhere; parent directories are created automatically if needed.

## Controls
- `q` or `Esc` — quit
- `d` — cycle decorations (underline → background → none)
- `m` — toggle table mode (DPS ↔ HEAL)
- `s` — toggle the settings pane
- `h` — open/close the encounter history panel
- `i` — when idle mode is active, toggle the idle overlay on/off to peek at the last encounter
- `↑/↓` — move the selection inside the settings pane
- `←/→` — adjust the selected setting (idle timeout, default decoration, default mode)

## Technical Notes & Behavior
- Party‑only: rows are filtered to common job codes (PLD/WAR/DRK/GNB, WHM/SCH/AST/SGE, MNK/DRG/NIN/SAM/RPR/VPR, BRD/MCH/DNC, BLM/SMN/RDM/PCT, BLU).
- Normalization: numeric fields arrive as strings; commas/percent signs are stripped before parsing for sorting/ratios. Damage share is computed from per‑combatant damage over encounter total.
- Encounter naming: while a fight is active some servers report generic names (e.g., "Encounter"); the header falls back to Zone until a final name is available.
- Background: widgets avoid setting a background color so your terminal theme (blur/transparency) stays visible. The header separator uses a subtle gray; background meters intentionally set a background for the meter fill only.
- Persisted config: settings are written to `~/.config/iinact-tui/iinact-tui.config` on Linux/macOS (or `%APPDATA%\iinact-tui\iinact-tui.config` on Windows). Set `IINACT_TUI_CONFIG_DIR` to override.
- History panel: press `h` to switch into the history view; use `↑/↓` or mouse scroll to pick a date, hit `Enter`/click to drill into the encounters list, press `Enter` again for per-encounter details, and `←`/`Backspace` to step back. Date and encounter lists load from lightweight indexes first, with overlay indicators while data hydrates; encounter detail fetches the full frame-by-frame record on demand.
- Idle overlay: when the app is idle you’ll see the idle window by default—press `i` to hide/show it without leaving idle mode so you can review the most recent encounter quickly.

## Troubleshooting
- Confirm IINACT is running and the endpoint is reachable. The default is `ws://127.0.0.1:10501/ws`.
- History or live table is empty? Only party and combat jobs are shown; pets/limit break lines are filtered out. (for now)

## Roadmap (short)
- "Dungeon Mode" to merge Encounters in the history for the same dungeon.
- "Dungeon View" to view historical dungeon runs with more information.
- Dedicated Limit Break window (who/when/how much/what level)
- Theme presets (purple/cyberpunk, monochrome, gray meters).
- Toggle for background opacity
- Persist meter mode and layout preference.

## License
This project does not currently declare a license. Ask before redistributing.
