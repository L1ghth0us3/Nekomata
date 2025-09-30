# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by Keep a Changelog and uses calendar dates (YYYY‑MM‑DD).

## [0.3.0] - Unreleased
- Rebranded the project as **Nekomata**, updating crate/binary names, docs, and visuals while preserving compatibility with existing IINACT-powered workflows.
- Introduced new configuration and history paths (`~/.config/nekomata`) and environment variables (`NEKOMATA_CONFIG_DIR`, `NEKOMATA_DUNGEON_CATALOG`).
- Removed the pre-release legacy fallback logic for `iinact-tui` paths and environment variables to streamline the rebrand.
- Added a heal view and toggle to history window.
- Added sorting and graph update for heal live view.
- Refactored the model.rs into dedicated submodules (`history_panel`, `settings`, `state`, `types`, `view`)
- Refactored the UI renderer into dedicated submodules (`header`, `status`, `settings`, `table`) to simplify future tweaks and keep rendering components focused.
- Split the history subsystem into `types`, `store`, and `recorder` modules with a thin facade so persistence data, sled access, and async recording responsibilities stay isolated.
- Reworked history persistence to store per-date and per-encounter summaries for fast indexed loading while preserving every CombatData frame.
- History panel now hydrates data lazily with loading indicators for dates, encounters, and detail views.
- Added `i` hotkey (idle-only) to toggle the idle overlay so you can peek at the most recent encounter without leaving idle mode.

## [0.2.0] - 2025-09-21

Highlights
- Added a modal settings pane (`s`) that lets you adjust configurable options without leaving the meter.
- Idle detection now surfaces in the footer as “Connected (idle)” once no active combat has been seen for the configured timeout.
- Idle timeout is user-adjustable with `↑/↓` while the settings pane is open and persists between runs.
- Configuration is stored as JSON under `~/.config/nekomata/nekomata.config` (override via `NEKOMATA_CONFIG_DIR`; Windows uses `%APPDATA%\nekomata`).
- Generalized status colors: idle shows dark orange, disconnect shows red.
- New configuration options allow choosing the default decoration style and opening mode; adjustments apply immediately and persist.

Controls
- `s`: toggle settings pane.
- `↑/↓`: move the selection inside the settings pane.
- `←/→`: adjust the highlighted setting.
- `h`: open the history panel (historic data)
- `m`: toggle DPS ↔ HEAL table mode.
- `d`: cycle table decorations (underline → background → none).
- `q` / `Esc`: quit.

## [0.1.0] - 2025-09-20
Initial MVP of the Nekomata terminal DPS meter for the IINACT plugin (ratatui).

Highlights
- Auto‑connects to IINACT at `ws://127.0.0.1:10501/ws` and subscribes to `CombatData` + `LogLine`.
- Party‑only rows (filters to known FFXIV jobs); case‑insensitive keys and numeric normalization.
- Live table with kagerou‑inspired columns: Name, Job, ENCDPS, Crit%, DH%, Deaths.
- Right‑aligned numeric headers and values; responsive column set based on terminal width.
- Two‑line per‑entry bars (meter:off): thin role‑colored bar directly under each entry (tank=75, healer=41, dps=124).
- Background meter mode (meter:on): compact one‑line rows with a role‑colored background fill proportional to ENCDPS.
- Header: Encounter/Zone on the first line, Dur | ENCDPS | Damage on the second; dim gray separator under the table header.
- Preserves terminal background (no forced background colors in normal widgets).

Keys
- `q`/`Esc`: quit
- `u`: toggle meter mode (off=underline bars, on=background meters)

Bug fixes and polish
- Ensured header separator always renders (all widths).
- Encounter title stays reactive during active fights (falls back to Zone if “Encounter”/empty).
- Removed experimental gradient bars; simplified to solid role colors for clarity.
