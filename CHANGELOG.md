# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by Keep a Changelog and uses calendar dates (YYYY‑MM‑DD).

## [Unreleased]

### History Settings
- Nested **History Settings** pane (from main Settings) to enable/disable recording, set retention limits, back up the database, browse archives read-only, and delete live history with confirmation.
- Retention limits (`None`, older than N days, max size MB) use draft-then-apply: destructive pruning requires explicit confirmation before any data is removed.
- Disabling history stops recording without deleting existing data; re-enabling reconnects to the same database.

## [0.5.0] - 2026-08-27

Highlights
- **File-based themes**: the TUI loads `.theme` TOML palettes from a `themes/` folder next to the binary (falls back to a built-in Synth Wave palette when none are found).
- **Bundled presets**: Synth Wave (default), Abyss Protocol, Catppuccin Latte, Catppuccin Mocha, Gruvbox Retro, and Tokyo Night.
- **Role-color toggle**: settings can keep the classic xterm-256 tank/healer/DPS meter colors or use the active theme's role colors.
- **Limit Break tracking**: parses ability lines so LB damage and caster can be shown as a dedicated panel, as a DPS table row, or hidden.
- **Prebuilt binaries**: GitHub Releases now ship Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) archives that include the `themes/` folder.

This tagged release also includes the untagged 0.4.0 work that was already on `main` (idle-while-disconnected, pre-job classes). See the 0.4.0 section below.

Settings & config
- New settings fields: Limit break display (`Off` / `PanelAlways` / `TableRow`), Theme, and Change role specific colors.
- Config keys `theme_id`, `role_theme_enabled`, and `limit_break_mode` persist across restarts. Older `show_limit_break` values are migrated (`true` → panel, `false` → off).

Controls
- Settings pane (`s`): `↑/↓` still move the selection; `←/→` cycle theme, role colors, and LB display in addition to the existing options.
- History panel: `Tab` (or `t`) switches between encounter history and dungeon-run history.

Architecture
- History loading moved into a dedicated `history/loader` task path; encounter detail rendering is shared between live and history views.
- UI modules live under `src/ui/` (`history`, `idle`, `lb`, `encounter_detail`).
- Limit Break uses a single tracker that attributes multi-target ability-line damage.

Distribution
- `themes/` must sit next to the executable. Prebuilt archives package it that way; `cargo run` from the repo root uses `./themes`.
- Dungeon catalog remains embedded in the binary (`NEKOMATA_DUNGEON_CATALOG` still overrides it).

Bug fixes
- Corrected the Synth Wave DRG hex color (`#8CA0FF`) so Dragoon job tint loads from the theme file.

## [0.4.0] - 2025-12-27

Highlights
- Idle mode now works even when disconnected from IINACT, showing "Disconnected (idle)" status after the configured timeout.
- Added support for pre-job classes (GLD, PGL, MRD, LNC, ARC, CNJ, THM, ROG) with proper color coding and role assignments.

Bug fixes
- Fixed typo in pre-job handling that prevented Gladiator (GLD) from rendering correctly.

UI improvements
- Removed redundant settings information from the header for a cleaner display.

## [0.3.0] - 2025-12-11

Highlights
- Rebranded the project as **Nekomata**, updating crate/binary names, docs, and visuals while preserving compatibility with existing IINACT-powered workflows.
- Introduced new configuration and history paths (`~/.config/nekomata`) and environment variables (`NEKOMATA_CONFIG_DIR`, `NEKOMATA_DUNGEON_CATALOG`).
- Removed the pre-release legacy fallback logic for `iinact-tui` paths and environment variables to streamline the rebrand.
- Added **Dungeon Mode**: toggleable mode that aggregates encounters into single dungeon runs while preserving individual encounter details.
- Added a heal view and toggle to history window.
- Added sorting and graph update for heal live view.
- Major code refactoring with modular architecture for improved maintainability.

Architecture & Code Quality
- Refactored the model.rs into dedicated submodules (`history_panel`, `settings`, `state`, `types`, `view`)
- Refactored the UI renderer into dedicated submodules (`header`, `status`, `settings`, `table`) to simplify future tweaks and keep rendering components focused.
- Split the history subsystem into `types`, `store`, and `recorder` modules with a thin facade so persistence data, sled access, and async recording responsibilities stay isolated.
- Reworked history persistence to store per-date and per-encounter summaries for fast indexed loading while preserving every CombatData frame.
- History panel now hydrates data lazily with loading indicators for dates, encounters, and detail views.
- Improved git setup with `.gitattributes` for consistent line endings and binary file handling.
- Enhanced `.gitignore` with comprehensive Rust and IDE patterns.
- Added `Cargo.lock` to repository for reproducible builds (binary application).

Controls
- `i`: when idle mode is active, toggle the idle overlay on/off to peek at the most recent encounter without leaving idle mode.
- `Shift-D`: when dungeon mode is active, manually cut off a dungeon run and save it.

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
