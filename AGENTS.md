# AGENTS.md

## Goal
Provide a minimal, language-agnostic description of how to consume **IINACT** (https://github.com/marzent/IINACT)
from a client to build custom visualizations (e.g., Rust + ratatui).

## WebSocket Endpoint
- Default: `ws://127.0.0.1:10501/ws`
- Transport: plain WebSocket (no auth).

## Protocol Overview (OverlayPlugin-compatible)
IINACT implements the OverlayPlugin WebSocket API used by ACT overlays.
Two interaction styles exist:

1. **Handler calls** (request/response): send a JSON object with a `call` field, receive one JSON reply.
   - Example: `{"call":"getLanguage"}` → `{"language":"English","languageId":"1","region":"Global","regionId":"1"}`

2. **Event subscription** (server push): send `{"call":"subscribe","events":[...EventNames...]}` to begin receiving streaming events.
   - Common events to subscribe to:
     - `CombatData` – encounter summary plus per-combatant stats
     - `LogLine` – raw log lines

### Typical `CombatData` payload
```json
{
  "type": "CombatData",
  "Encounter": {
    "title": "Encounter Name",
    "duration": "02:34",
    "encdps": "12345",
    "damage": "987654",
    "CurrentZoneName": "Zone Name"
  },
  "Combatant": {
    "Alice": { "Job":"NIN", "encdps":"4567", "crithit%":"21%", "DirectHit%":"28%", "deaths":"0" },
    "Bob":   { "Job":"WHM", "encdps":"1234", "crithit%":"12%", "DirectHit%":"5%",  "deaths":"1" }
  },
  "isActive": "true"
}
```
Notes:
- Numeric values often arrive as **strings** and may contain commas; normalize before sorting/aggregating.
- Keys can differ by **case** across implementations (`encdps` vs `ENCDPS`); prefer case-insensitive lookup.

### Typical `LogLine` payload
```json
{ "type": "LogLine", "line": "21|2025-01-01T12:34:56.789|..." }
```

## Reference Client (Python)
See `query_iinact.py` for a minimal, dependency-light client that:
- Connects to the WS
- Performs a handler call (`getLanguage`)
- Subscribes to `CombatData` and `LogLine`
- Pretty-prints an encounter table, sorted by ENCDPS

### Running
```bash
pip install websockets
python3 query_iinact.py --ws ws://127.0.0.1:10501/ws
# Exit after the first CombatData:
python3 query_iinact.py --once
# Show LogLine summaries too:
python3 query_iinact.py --show-logline
```

## Guidance for Agents (Nekomata TUI target)
- Use a WS client (e.g., `tokio-tungstenite`) to connect to `ws://127.0.0.1:10501/ws`.
- Send `{"call":"getLanguage"}` to verify connectivity.
- Send `{"call":"subscribe","events":["CombatData","LogLine"]}` to begin streaming.
- Maintain a state struct with the latest `Encounter` plus a map of combatants.
- Normalize numeric strings to floats (strip commas and percent signs).
- Present a live table sorted by ENCDPS; refresh on each incoming `CombatData`.
- Optional: filter out `isActive == "false"` to avoid stale snapshots.
- Track encounter activity timestamps so the UI can surface an idle state when no fights are active for the configured timeout.
- Surface user-facing settings through a modal pane and persist them to disk so inputs survive restarts.

### Current TUI Behavior (v0.2.0)
- Rendering
  - Table columns: Name, Share%, ENCDPS, Job, Crit%, DH%, Deaths (numeric columns are right‑aligned). On narrow widths, Share% survives longer than ENCDPS/Job.
  - Responsive breakpoints hide columns at narrow widths (down to Name‑only).
  - Header: line 1 shows Encounter/Zone; line 2 shows Dur | ENCDPS | Damage; a dim gray separator appears under the table header.
  - Party‑only rows using a known job set; case‑insensitive key lookup.
- Decorations (pluggable)
  - `Decor: underline` (default): two-line rows with a thin role-colored bar directly under each entry.
  - `Decor: background`: one-line rows with a role-colored background meter behind each entry.
  - `Decor: none`: no additional row decoration; compact one-line rows.
  - Cycle key: `d`.
- Modes & status
  - `m` toggles between DPS and healing views.
  - Idle indicator flips the footer to “Connected (idle)” after the configured timeout; disconnected shows red.
- Settings & persistence
  - `s` opens a modal settings pane; `↑/↓` moves the selection, `←/→` adjusts the highlighted value.
  - Idle timeout accepts `0` to disable idle mode; defaults for decoration and opening mode are configured here and persist to disk.
- Settings persist to `~/.config/nekomata/nekomata.config` (override with `NEKOMATA_CONFIG_DIR`; Windows uses `%APPDATA%\nekomata`).
- Styling
  - Foreground-only for normal widgets to preserve terminal blur/transparency. Background is used only for the meter fill.
- Role colors (xterm-256): tank=75, healer=41, dps=124; job name text uses per-job colors.

## Encounter History (sled-backed)
- Storage lives under the same config root, inside `history/encounters.sled`; override via `NEKOMATA_CONFIG_DIR` like the main config file.
- `HistoryStore` wraps sled trees; keys use `enc::<ms_since_epoch>::<id>` ordering so new namespaces can be added without migrations.
- `spawn_recorder` starts a background task fed by `RecorderHandle`; push snapshots with `record_components(encounter, rows, raw_json)` and call `flush()` when tearing down connections.
- Records capture first/last seen timestamps, the final encounter summary, combatant rows, and the last raw JSON payload. Empty passive snapshots are skipped to avoid noise.
- The interface exposes helpers (`remove`, `tree`, `HistoryKey::prefix`) to make adding/removing sled namespaces straightforward for future features.
- TUI access: hit `h` to enter the history panel. The first view lists dates; `↑/↓` or mouse scroll move the selection, and `Enter`/left-click drill into the encounter list. Press `Enter` again to open the encounter detail pane, use `←` to back out, and `h`/`Esc` to exit entirely.
