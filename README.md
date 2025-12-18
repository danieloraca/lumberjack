# 🪓 Lumberjack
![Rust](https://github.com/danieloraca/lumberjack/actions/workflows/test.yml/badge.svg)

**Lumberjack** is a terminal UI (TUI) for browsing and searching **AWS CloudWatch Logs**.

It lets you:
- Browse and fuzzy-search log groups
- Filter logs by time range and pattern
- Stream and scroll results
- Pretty-print embedded JSON logs
- Stay entirely in the terminal

Built in **Rust**, powered by **ratatui**, **crossterm**, and the **AWS SDK for Rust**.

---

## Features

- 📂 Log group browser (scrollable, with `/` fuzzy search)
- 🔍 Filter logs by:
  - Start time
  - End time
  - Filter pattern
  - JSON fields via shorthand (e.g. `routing_id=123` → `{ $.routing_id = 123 }`)
- ⏱ Time parsing with friendly input
  - Absolute: `2025-12-11T10:00:00Z` or `2025-12-11 10:00:00`
  - Relative: `-5m`, `-1h`, `-1d` (relative to now)
- 🧾 Pretty-printed JSON output
- 📜 Scrollable results with scrollbar
- ⌨️ Keyboard-driven UI
- 🌑 Focus-aware panes (Groups / Filter / Results)

---

## Requirements

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
- `t` – Toggle tail/stream mode for results
- `Esc` – Cancel editing or group search
- `q` – Quit (except while editing or in group search)
