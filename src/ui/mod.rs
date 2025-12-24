mod results;
mod styles;

use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Widget};

use crate::app::{App, FilterField, Focus};

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let header_style = styles::header();
        let footer_style = styles::footer();

        let groups_block_style = styles::groups_block(self.focus == Focus::Groups);
        let filter_block_style = styles::filter_block(self.focus == Focus::Filter);
        let results_block_style = styles::results_block(self.focus == Focus::Results);

        let groups_item_style = styles::group_item(self.focus == Focus::Groups);
        let groups_selected_style = styles::groups_selected(self.focus == Focus::Groups);

        let groups_border = styles::pane_border(self.focus == Focus::Groups);
        let filter_border = styles::pane_border(self.focus == Focus::Filter);
        let results_border = styles::pane_border(self.focus == Focus::Results);

        buf.set_style(chunks[0], header_style);
        buf.set_style(chunks[3], footer_style);

        let header =
            Layout::horizontal([Constraint::Length(20), Constraint::Min(20)]).split(chunks[0]);
        let footer =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]).split(chunks[3]);
        let groups_row =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[1]);

        let header_right_text: String = format!(
            "Profile: {} | Region: {}",
            self.profile.as_str(),
            self.region.as_str(),
        );
        Line::from(self.app_title.as_str())
            .bold()
            .render(header[0], buf);
        Line::from(header_right_text)
            .right_aligned()
            .style(header_style)
            .render(header[1], buf);

        let footer_left = if let Some(msg) = &self.status_message {
            msg.clone()
        } else if self.group_search_active {
            format!("Search groups: {}", self.group_search_input)
        } else {
            "Tab Switch pane  ↑↓ Move  Enter Edit/Run  t Tail  y Copy  Esc Cancel  q Quit"
                .to_string()
        };

        // Tail indicator on the right, next to version
        let footer_right = if self.tail_mode {
            format!("[Tailing] {}", env!("CARGO_PKG_VERSION"))
        } else {
            env!("CARGO_PKG_VERSION").to_string()
        };

        Line::from(footer_left)
            .style(footer_style)
            .render(footer[0], buf);

        Line::from(footer_right)
            .right_aligned()
            .style(footer_style)
            .render(footer[1], buf);

        let groups_block = Block::bordered()
            .title("Groups")
            .style(groups_block_style)
            .border_style(groups_border);

        let inner = groups_block.inner(groups_row[0]);
        groups_block.render(groups_row[0], buf);

        let filter_block = Block::bordered()
            .title("Filter")
            .style(filter_block_style)
            .border_style(filter_border);

        let filter_inner = filter_block.inner(groups_row[1]);
        filter_block.render(groups_row[1], buf);

        let results_block = Block::bordered()
            .title("Results")
            .style(results_block_style)
            .border_style(results_border);

        let results_inner = results_block.inner(chunks[2]);
        results_block.render(chunks[2], buf);

        let visible_rows = inner.height as usize;
        let start = self.groups_scroll;
        let end = (start + visible_rows).min(self.groups.len());

        for (row, idx) in (start..end).enumerate() {
            let group = &self.groups[idx];

            let selected = idx == self.selected_group;
            let marker = if selected { "(●) " } else { "( ) " };

            let y = inner.y + row as u16;
            Line::from(format!("{marker}{group}"))
                .style(if selected {
                    groups_selected_style
                } else {
                    groups_item_style
                })
                .render(
                    Rect {
                        x: inner.x,
                        y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );
        }

        if self.searching && self.lines.is_empty() {
            let dots = ".".repeat(self.dots);
            let msg = format!("Searching{dots}");

            Line::from(msg).style(styles::default_gray()).render(
                Rect {
                    x: results_inner.x,
                    y: results_inner.y,
                    width: results_inner.width,
                    height: 1,
                },
                buf,
            );

            // stop here so we don't render stale lines underneath
            return;
        }

        // Call the refactored renderer
        self.render_results(results_inner, buf);

        let mut row_y = filter_inner.y;

        let field_style = |field: FilterField| {
            let active = self.focus == Focus::Filter && field == self.filter_field;
            styles::filter_field(active, active && self.editing)
        };

        let line = |label: &str, value: &str| format!("{label}: {value}");

        Line::from(line("Start", &self.filter_start))
            .style(field_style(FilterField::Start))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        Line::from(line("End", &self.filter_end))
            .style(field_style(FilterField::End))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        Line::from(line("Query", &self.filter_query))
            .style(field_style(FilterField::Query))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        // ---- fake blinking cursor inside the active filter field ----
        if self.focus == Focus::Filter && self.editing && self.cursor_on {
            // Which row is the active field on?
            //
            // NOTE: The presets hint is non-interactive; only the text fields and
            // the Search button participate in cursor positioning.
            let field_row = match self.filter_field {
                FilterField::Start => 0,
                FilterField::End => 1,
                FilterField::Query => 2,
                FilterField::Search => 3, // mapped to the Search button row
            };

            // Only show cursor for text fields
            if self.filter_field != FilterField::Search {
                let label = match self.filter_field {
                    FilterField::Start => "Start: ",
                    FilterField::End => "End: ",
                    FilterField::Query => "Query: ",
                    FilterField::Search => "",
                };

                let value_len = self.active_field_len();
                let y = filter_inner.y + field_row;

                // Clamp cursor pos to field length
                let cursor_col = self.filter_cursor_pos.min(value_len);

                // Cursor x = left + label width + cursor_col
                let mut x = filter_inner.x + label.len() as u16 + cursor_col as u16;

                // clamp within the filter box
                let max_x = filter_inner.x + filter_inner.width.saturating_sub(1);
                if x > max_x {
                    x = max_x;
                }

                // draw a vertical bar cursor
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('▏').set_style(styles::cursor());
                }
            }
        }

        // "button"
        let btn = "[ Search ]";
        Line::from(btn)
            .style(field_style(FilterField::Search))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        // Presets hint (non-interactive) — intentionally subdued at the bottom of the pane
        let presets_text = " Presets: 1 = -5m  2 = -15m  3 = -1h  4 = -24h ";

        // Right-align the presets hint within the filter pane
        let text_width = presets_text.len() as u16;
        let pane_width = filter_inner.width;
        let presets_x = filter_inner.x + pane_width.saturating_sub(text_width);

        Line::from(presets_text)
            .style(styles::presets_hint())
            .render(
                Rect {
                    x: presets_x,
                    y: row_y,
                    width: text_width.min(pane_width),
                    height: 1,
                },
                buf,
            );

        // --- Save / Load filter popups (drawn on top of everything else) ---
        if self.save_filter_popup_open {
            // Centered 40x5 popup
            let popup_width = 40u16.min(area.width);
            let popup_height = 5u16.min(area.height);
            let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
            let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

            let popup_area = Rect {
                x: popup_x,
                y: popup_y,
                width: popup_width,
                height: popup_height,
            };

            let block = Block::bordered()
                .title("Save filter")
                .style(styles::popup_block())
                .border_style(styles::popup_border());
            let inner = block.inner(popup_area);
            block.render(popup_area, buf);

            // Label + current name on the next line
            let label = "Name:";
            Line::from(label)
                .style(Style::default().fg(Color::White))
                .render(
                    Rect {
                        x: inner.x,
                        y: inner.y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );

            let name_line = format!("{}", self.save_filter_name);
            Line::from(name_line).style(styles::popup_border()).render(
                Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );

            // Hint line
            Line::from("Enter Save   Esc Cancel")
                .style(Style::default().fg(Color::Gray))
                .render(
                    Rect {
                        x: inner.x,
                        y: inner.y + 3.min(inner.height.saturating_sub(1)),
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );
        }

        if self.load_filter_popup_open {
            // Centered popup sized to number of filters (up to a max height)
            let popup_width = 40u16.min(area.width);
            let max_height = 10u16;
            let needed_height = (self.saved_filters.len() as u16 + 3).max(3);
            let popup_height = max_height.min(needed_height).min(area.height);
            let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
            let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

            let popup_area = Rect {
                x: popup_x,
                y: popup_y,
                width: popup_width,
                height: popup_height,
            };

            let block = Block::bordered()
                .title("Load filter")
                .style(styles::popup_block())
                .border_style(styles::popup_border());
            let inner = block.inner(popup_area);
            block.render(popup_area, buf);

            // Render filter names with a simple highlight on the selected one
            let mut y = inner.y;
            for (idx, f) in self.saved_filters.iter().enumerate() {
                if y >= inner.y + inner.height {
                    break;
                }

                let marker = if idx == self.load_filter_selected {
                    ">"
                } else {
                    " "
                };
                let line = format!("{marker} {}", f.name);
                let style = if idx == self.load_filter_selected {
                    styles::popup_border()
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(line).style(style).render(
                    Rect {
                        x: inner.x,
                        y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );

                y += 1;
            }

            // Hint line at the bottom of the popup
            Line::from("Enter Load   Esc Cancel")
                .style(styles::default_gray())
                .render(
                    Rect {
                        x: inner.x,
                        y: inner.y + inner.height.saturating_sub(1),
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );
        }
    }
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, mpsc};
    use std::time::Instant;

    fn make_app() -> App {
        let groups_owned = vec!["g1".to_string(), "g2".to_string()];
        let (tx, rx) = mpsc::channel();

        App {
            app_title: "lumberjack".to_string(),
            exit: false,
            lines: vec![],
            filter_cursor_pos: 0,

            all_groups: groups_owned.clone(),
            groups: groups_owned,
            selected_group: 0,
            groups_scroll: 0,

            profile: "test".to_string(),
            region: "eu-west-1".to_string(),
            focus: Focus::Filter,

            filter_start: "".to_string(),
            filter_end: "".to_string(),
            filter_query: "".to_string(),
            filter_field: FilterField::Query,
            editing: false,
            cursor_on: true,
            last_blink: Instant::now(),

            group_search_active: false,
            group_search_input: "".to_string(),

            search_tx: tx,
            search_rx: rx,
            searching: false,
            dots: 0,
            last_dots: Instant::now(),
            results_scroll: 0,

            tail_mode: false,
            tail_stop: Arc::new(AtomicBool::new(false)),
            status_message: None,
            status_set_at: None,

            saved_filters: Vec::new(),
            save_filter_popup_open: false,
            save_filter_name: String::new(),
            load_filter_popup_open: false,
            load_filter_selected: 0,
        }
    }

    fn buffer_contains_symbol(buf: &Buffer, sym: &str) -> bool {
        buf.content().iter().any(|c| c.symbol() == sym)
    }

    fn buffer_contains_text(buf: &Buffer, needle: &str) -> bool {
        // crude but works: join all symbols and search
        let screen: String = buf
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<Vec<_>>()
            .join("");
        screen.contains(needle)
    }

    #[test]
    fn draws_cursor_when_editing_active_text_field() {
        let mut app = make_app();
        app.focus = Focus::Filter;
        app.editing = true;
        app.cursor_on = true;
        app.filter_field = FilterField::Query;
        app.filter_query = "abc".to_string();

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        assert!(buffer_contains_symbol(&buf, "▏"), "expected cursor ▏");
    }

    #[test]
    fn does_not_draw_cursor_when_not_editing() {
        let mut app = make_app();
        app.focus = Focus::Filter;
        app.editing = false;
        app.cursor_on = true;
        app.filter_field = FilterField::Query;

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        assert!(
            !buffer_contains_symbol(&buf, "▏"),
            "cursor should not be drawn when not editing"
        );
    }

    #[test]
    fn does_not_draw_cursor_on_search_button() {
        let mut app = make_app();
        app.focus = Focus::Filter;
        app.editing = true;
        app.cursor_on = true;
        app.filter_field = FilterField::Search;

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        assert!(
            !buffer_contains_symbol(&buf, "▏"),
            "cursor should not be drawn when Search is selected"
        );
    }

    #[test]
    fn shows_searching_message_when_searching_and_no_lines() {
        let mut app = make_app();
        app.searching = true;
        app.dots = 3;
        app.lines.clear(); // must be empty to trigger the early-return path

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        assert!(
            buffer_contains_text(&buf, "Searching..."),
            "expected Searching... message"
        );
    }

    #[test]
    fn shows_group_search_prompt_and_input_in_footer() {
        let mut app = make_app();
        app.focus = Focus::Groups;
        app.group_search_active = true;
        app.group_search_input = "api".to_string();

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        assert!(
            buffer_contains_text(&buf, "Search groups: api"),
            "expected footer to show 'Search groups: api'"
        );
    }

    #[test]
    fn shows_tail_indicator_in_footer_when_tail_mode_on() {
        let mut app = make_app();
        app.tail_mode = true;

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        assert!(
            buffer_contains_text(&buf, "[Tailing]"),
            "expected footer to show '[Tailing]' when tail_mode is on"
        );
    }

    #[test]
    fn shows_time_presets_hint_in_filter_pane() {
        let mut app = make_app();
        app.focus = Focus::Filter;

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        // The filter pane is narrow; the hint may be truncated by layout.
        // Assert a stable prefix rather than the full string.
        assert!(
            buffer_contains_text(&buf, "Presets:"),
            "expected presets hint to be rendered in filter pane"
        );
    }

    #[test]
    fn presets_hint_is_rendered_with_subdued_style() {
        let mut app = make_app();
        app.focus = Focus::Filter;

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        // Instead of searching text in the full-screen buffer (which can match the header/footer),
        // compute the exact cell coordinates for the presets hint row inside the Filter pane.
        //
        // Layout mirrors the render() function:
        // - Vertical: header(1), top row(6), results(min), footer(1)
        // - Top row: groups 60%, filter 40%
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let groups_row =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[1]);

        // Filter block inner rect
        let filter_block = Block::bordered().title("Filter");
        let filter_inner = filter_block.inner(groups_row[1]);

        // Presets line is rendered after Start, End, Query, and the Search button.
        // It lives on row index 4 (0-based) within the filter_inner.
        let presets_y = filter_inner.y + 4;
        let presets_x = filter_inner.x;

        let cell = buf
            .cell((presets_x, presets_y))
            .expect("expected presets cell to exist in buffer");
        let style = cell.style();

        assert_eq!(
            style.fg,
            Some(Color::Rgb(50, 50, 50)),
            "expected presets hint to have subdued foreground color"
        );

        assert_eq!(
            style.bg,
            Some(Color::Rgb(20, 20, 20)),
            "expected presets hint to have subdued background color"
        );
    }

    #[test]
    fn results_renders_report_line_without_corrupting_tokens() {
        let mut app = make_app();
        app.lines.clear();
        app.lines.push(
            "2025-12-22T21:25:28.694+00:00 REPORT RequestId: TEST \
             Duration: 13269.00 ms\tBilled Duration: 13269 ms\tMemory Size: 1024 MB\tMax Memory Used: 272 MB"
                .to_string(),
        );
        app.focus = Focus::Results;

        let area = Rect::new(0, 0, 120, 10);
        let mut buf = Buffer::empty(area);

        (&app).render(area, &mut buf);

        // Assert key tokens are present
        assert!(
            !buffer_contains_text(&buf, "BDuration"),
            "should not render 'BDuration' artifact"
        );
        assert!(
            !buffer_contains_text(&buf, "MSize"),
            "should not render 'MSize' artifact"
        );
        assert!(
            !buffer_contains_text(&buf, "MaxMemory"),
            "should not render 'MaxMemory' artifact"
        );
    }

    #[test]
    fn results_renders_info_line_with_tabs_without_merging_tokens() {
        let mut app = make_app();

        app.lines.clear();
        app.lines.push(
            "2025-12-23T15:02:15.620+00:00 2025-12-23T15:02:15.620Z\tea080ace-0f99-4021-a683-0599cfea7c45\tINFO\tThere are 11 messages in the queue, starting 3 tasks"
                .to_string(),
        );
        app.focus = Focus::Results;

        let area = Rect::new(0, 0, 120, 10);
        let mut buf = Buffer::empty(area);
        (&app).render(area, &mut buf);

        // We only assert that the known-bad merged artifact is gone.
        // Layout in this small test area may clip or wrap the full sentence.
        assert!(
            !buffer_contains_text(&buf, "Iare"),
            "should not render 'Iare' artifact"
        );
    }
}
