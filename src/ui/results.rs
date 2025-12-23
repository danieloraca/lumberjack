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

        // Draw scrollbar once per frame
        App::draw_scrollbar(
            buf,
            results_inner,
            start, // first visible line index
            total, // total number of lines
            self.focus == crate::app::Focus::Results,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, mpsc};
    use std::time::Instant;

    use crate::app::{App, FilterField, Focus};

    fn make_results_app(lines: Vec<&str>) -> App {
        let (tx, rx) = mpsc::channel();
        App {
            app_title: "Test".to_string(),
            exit: false,
            lines: lines.into_iter().map(|s| s.to_string()).collect(),
            filter_cursor_pos: 0,

            all_groups: vec![],
            groups: vec![],
            selected_group: 0,
            groups_scroll: 0,

            profile: "test".to_string(),
            region: "eu-west-1".to_string(),
            focus: Focus::Results,

            filter_start: String::new(),
            filter_end: String::new(),
            filter_query: String::new(),
            filter_field: FilterField::Query,
            editing: false,
            cursor_on: true,
            last_blink: Instant::now(),

            group_search_active: false,
            group_search_input: String::new(),

            search_tx: tx,
            search_rx: rx,
            searching: false,
            dots: 0,
            last_dots: Instant::now(),
            results_scroll: 0,

            tail_mode: false,
            tail_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),

            status_message: None,
            status_set_at: None,

            // JSON popup defaults
            json_popup_open: false,
            json_popup_content: String::new(),

            saved_filters: Vec::new(),
            save_filter_popup_open: false,
            save_filter_name: String::new(),
            load_filter_popup_open: false,
            load_filter_selected: 0,
        }
    }

    fn buffer_to_string(buf: &Buffer, area: Rect) -> String {
        let mut out = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let ch = buf
                    .cell((x, y))
                    .map(|c| c.symbol())
                    .unwrap_or(" ")
                    .chars()
                    .next()
                    .unwrap_or(' ');
                out.push(ch);
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn timestamp_coloring_preserves_message_text() {
        let app = make_results_app(vec![
            "2025-12-22T21:25:28.694+00:00 REPORT RequestId: TEST Duration: 10 ms Billed Duration: 11 ms Memory Size: 1024 MB Max Memory Used: 272 MB",
        ]);

        // Wide enough area so the whole line fits on one row.
        let area = Rect::new(0, 0, 160, 3);
        let results_inner = area;
        let mut buf = Buffer::empty(area);

        app.render_results(results_inner, &mut buf);

        let rendered = buffer_to_string(&buf, area);

        assert!(
            rendered.contains("Duration: 10 ms"),
            "expected 'Duration: 10 ms' in rendered output, got:\n{}",
            rendered
        );
        assert!(
            rendered.contains("Billed Duration: 11 ms"),
            "expected 'Billed Duration: 11 ms' in rendered output, got:\n{}",
            rendered
        );
        assert!(
            rendered.contains("Memory Size: 1024 MB"),
            "expected 'Memory Size: 1024 MB' in rendered output, got:\n{}",
            rendered
        );
        assert!(
            rendered.contains("Max Memory Used: 272 MB"),
            "expected 'Max Memory Used: 272 MB' in rendered output, got:\n{}",
            rendered
        );
    }

    #[test]
    fn tabs_are_expanded_without_merging_words() {
        let app = make_results_app(vec![
            "2025-12-23T15:02:15.620+00:00 2025-12-23T15:02:15.620Z\tea080ace-0f99-4021-a683-0599cfea7c45\tINFO\tThere are 11 messages in the queue, starting 3 tasks",
        ]);

        let area = Rect::new(0, 0, 160, 3);
        let mut buf = Buffer::empty(area);
        app.render_results(area, &mut buf);

        let rendered = buffer_to_string(&buf, area);

        // We don't enforce exact wrapping; just ensure we didn't reproduce the Iare bug.
        assert!(
            !rendered.contains("Iare"),
            "should not render 'Iare' artifact, got:\n{}",
            rendered
        );
        assert!(
            rendered.contains("INFO"),
            "expected 'INFO' token in rendered output, got:\n{}",
            rendered
        );
        assert!(
            rendered.contains("There are 11 messages in the queue"),
            "expected 'There are 11 messages in the queue' in rendered output, got:\n{}",
            rendered
        );
    }

    #[test]
    fn draws_scrollbar_when_multiple_lines() {
        // Enough lines to require scrolling
        let mut lines = Vec::new();
        for i in 0..20 {
            lines.push(format!("2025-12-22T21:25:{:02}.000+00:00 line {}", i, i));
        }
        let app = make_results_app(lines.iter().map(|s| s.as_str()).collect());

        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);

        app.render_results(area, &mut buf);

        // Rightmost column should contain at least one scrollbar thumb '█' or track '│'
        let x = area.x + area.width - 1;
        let mut has_scroll_glyph = false;
        for y in area.y..area.y + area.height {
            if let Some(cell) = buf.cell((x, y)) {
                let sym = cell.symbol();
                if sym == "│" || sym == "█" {
                    has_scroll_glyph = true;
                    break;
                }
            }
        }

        assert!(
            has_scroll_glyph,
            "expected scrollbar glyphs in rightmost column, but none were found"
        );
    }
}
