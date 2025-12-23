use ratatui::prelude::{Buffer, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::app::App;

impl App {
    pub fn render_results(&self, results_inner: Rect, buf: &mut Buffer) {
        // Leave 1 column for the scrollbar and 1 "guard" column before the border.
        let guard_w = 1u16;
        let scrollbar_w = 1u16;
        let reserved = guard_w + scrollbar_w;

        let text_area = Rect {
            x: results_inner.x,
            y: results_inner.y,
            width: results_inner.width.saturating_sub(reserved),
            height: results_inner.height,
        };

        if text_area.width == 0 || text_area.height == 0 {
            return;
        }

        // Flatten entries into raw lines (no manual wrapping).
        let mut raw_lines: Vec<String> = Vec::new();
        for entry in &self.lines {
            for raw_line in entry.lines() {
                raw_lines.push(raw_line.to_string());
            }
        }

        let total = raw_lines.len();
        let visible_rows = text_area.height as usize;

        if total == 0 {
            App::draw_scrollbar(
                buf,
                results_inner,
                0,
                0,
                self.focus == crate::app::Focus::Results,
            );
            return;
        }

        // Simple per-line vertical window
        let start = self.results_scroll.min(total.saturating_sub(1));
        let end = (start + visible_rows).min(total);

        for (i, line) in raw_lines[start..end].iter().enumerate() {
            let y = text_area.y + i as u16;

            let expanded = if line.contains('\t') {
                line.replace('\t', "    ")
            } else {
                line.clone()
            };

            // Heuristic: line starts with something RFC3339-ish, e.g. 2025-12-21T16:11:00+00:00
            let looks_like_ts = expanded.len() >= 20
                && expanded.chars().nth(4) == Some('-')
                && expanded.chars().nth(7) == Some('-')
                && expanded.chars().nth(10) == Some('T')
                && (expanded.ends_with('Z') || expanded.contains('+'));

            if looks_like_ts {
                // Take characters up to the first space as the timestamp prefix.
                let mut chars = expanded.chars().peekable();
                let mut ts = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' {
                        break;
                    }
                    ts.push(c);
                    chars.next();
                }

                // Everything after the timestamp (including the space if present)
                let rest: String = chars.collect();

                let ts_style = Style::default()
                    .fg(ratatui::style::Color::Rgb(100, 180, 180))
                    .bg(ratatui::style::Color::Rgb(5, 5, 5))
                    .add_modifier(ratatui::style::Modifier::BOLD);

                let spans = if rest.is_empty() {
                    vec![Span::styled(ts, ts_style)]
                } else {
                    vec![Span::styled(ts, ts_style), Span::raw(rest)]
                };

                Line::from(spans).render(
                    Rect {
                        x: text_area.x,
                        y,
                        width: text_area.width,
                        height: 1,
                    },
                    buf,
                );
            } else {
                // No special timestamp; render the whole line normally.
                Line::from(expanded.as_str()).render(
                    Rect {
                        x: text_area.x,
                        y,
                        width: text_area.width,
                        height: 1,
                    },
                    buf,
                );
            }
        }
    }
}
