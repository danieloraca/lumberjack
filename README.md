<img width="1024" height="1024" alt="image" src="https://github.com/user-attachments/assets/8d240a0a-61e2-4c01-8078-9f9b837d762b" />
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
- ⌨️ Keyboard-driven UI
  - `/` fuzzy-search groups
  - `1/2/3/4` for time presets
  - `t` to tail
  - `y` to copy all results
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
- `↑` / `↓` – Move selection / scroll
- `Enter` – Edit filter field / run search
- `1` / `2` / `3` / `4` – Quick time presets for **Start** (sets Start to `-5m` / `-15m` / `-1h` / `-24h`, and clears End to “now”)
- `s` – Save current filter (opens name popup; persists to `~/.config/lumberjack/filters.json`)
- `F` – Load saved filter (opens popup with saved filter names)
- `t` – Toggle tail/stream mode for results
- `Esc` – Cancel editing, group search, or close popups
- `y` – Copy all Results to clipboard (when Results pane is focused)
- `q` – Quit (except while editing or in group search)
