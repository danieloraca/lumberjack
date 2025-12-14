# 🪓 Lumberjack

**Lumberjack** is a terminal UI (TUI) for browsing and searching **AWS CloudWatch Logs**.

It lets you:
- Browse log groups
- Filter logs by time range and pattern
- Stream and scroll results
- Pretty-print embedded JSON logs
- Stay entirely in the terminal

Built in **Rust**, powered by **ratatui**, **crossterm**, and the **AWS SDK for Rust**.

---

## Features

- 📂 Log group browser (scrollable)
- 🔍 Filter logs by:
  - Start time
  - End time
  - Filter pattern
- ⏱ Time parsing with friendly input
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
