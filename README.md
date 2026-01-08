# 🪓 Lumberjack
![Rust](https://github.com/danieloraca/lumberjack/actions/workflows/test.yml/badge.svg)

**Lumberjack** is a 2026‑grade terminal UI (TUI) for browsing and searching **AWS CloudWatch Logs** — fast, keyboard‑driven, and unapologetically anti-click.

It lets you:
- Browse and fuzzy-search log groups like TV channels
- Filter logs by time range and pattern with human‑friendly input (`-5m`, `-30s`, `-1h`)
- Stream and scroll results in a proper `tail -f` style TUI
- Stay entirely in the terminal — no tabs, no spinners, no surprise “new UI” toggles

Built in **Rust**, powered by **ratatui**, **crossterm**, and the **AWS SDK for Rust**.

---

## Features (a.k.a. Why Not Just Use The Console?)

- 📂 Log group browser (scrollable, with `/` fuzzy search)  
  Flip through log groups like channels, without waiting for a web app to boot.
- 🔍 Filter logs by:
  - Start time
  - End time
  - Filter pattern
  - JSON fields via shorthand:
    - Single field: `routing_id=123` → `{ $.routing_id = 123 }`
    - Multiple fields: `routing_id=1364 task="batch-attendances"` → `{ $.routing_id = 1364 && $.task = "batch-attendances" }`
  - Saved presets:
    - Save current filter: `s` (give it a name; saved to `~/.config/lumberjack/filters.json`)
    - Load saved filter: `F` (open popup, select by name)
    - Treat them like log mixtapes: “last-hour-errors”, “weird-timeouts”, “that-one-tenant”.
- ⏱ Time parsing with friendly input
  - Absolute: `2025-12-11T10:00:00Z` or `2025-12-11 10:00:00`
  - Relative: `-30s`, `-5m`, `-1h`, `-1d` (relative to now)
- 🧾 JSON-friendly output
  - Keeps underlying log lines intact for copying
  - Designed to play nicely with large, structured payloads
- 📜 Scrollable results with a real scrollbar (no infinite-scroll roulette)
- 🔎 Result selection & popup detail view
  - Move a highlight through Results with `↑` / `↓`
  - Press `Enter` or `Space` on a selected line to open a **Result detail** popup
  - Popup shows the full line with word wrapping and vertical scrolling
  - When a JSON `"sql"` field is present, the popup shows the SQL query as a nicely formatted, multi-line block
  - Press `y` in the popup to copy the formatted SQL (or the selected line when not SQL) to the clipboard
- ⌨️ Keyboard-driven UI
  - `/` fuzzy-search groups
  - `1/2/3/4` for time presets
  - `t` to tail
  - `y` to copy all results
  - `T` to cycle color themes (Dark → Light → Green CRT)
- 🎨 Theme support
  - Dark (default)
  - Light
  - Retro Green CRT (phosphor-style, neon green on black)
- 🌑 Focus-aware panes (Groups / Filter / Results) with clear borders and styles

---

## Requirements (2026 Edition)

- Rust (stable)
- AWS credentials configured locally  
  (via `~/.aws/credentials`, environment variables, or SSO)

---

## Installation

```bash
git clone https://github.com/danieloraca/lumberjack.git
cd lumberjack
cargo build --release
```

---

## Running
```bash
cargo run -- --profile=<aws-profile> --region=<aws-region>
```

---

## Keybindings

- `Tab` – Switch between Groups / Filter / Results
- `/` – Fuzzy-search log groups (when Groups pane is focused)
- `↑` / `↓` – Move selection / scroll (and move the highlighted result row when Results pane is focused)
- `Enter` – Edit filter field / run search; in Results pane, opens Result detail popup for the highlighted line
- `1` / `2` / `3` / `4` – Quick time presets for **Start** (sets Start to `-5m` / `-15m` / `-1h` / `-24h`, and clears End to “now”)
- `s` – Save current filter (opens name popup; persists to `~/.config/lumberjack/filters.json`)
- `F` – Load saved filter (opens popup with saved filter names)
- `t` – Toggle tail/stream mode for results
- `T` – Cycle color themes (Dark → Light → Green CRT)
- `Esc` – Cancel editing, group search, or close popups
- `y` – Copy all Results to clipboard (when Results pane is focused and no popup is open)
- `Space` – In Results pane, open Result detail popup for the highlighted line
- `y` – In Result detail popup, copy the formatted SQL when a JSON `"sql"` field is present (otherwise copy the selected line); popup remains open
- `q` – Quit (except while editing or in group search)
