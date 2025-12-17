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

        let groups_item_style = if self.focus == Focus::Groups {
            Style::default().bg(Color::Black).fg(Color::White)
        } else {
            Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140))
        };

        let groups_selected_style = if self.focus == Focus::Groups {
            Style::default()
                .bg(Color::Rgb(40, 40, 40))
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().bg(Color::Rgb(18, 18, 18)).fg(Color::White) // still readable while unfocused
        };

        let header_style = Style::default().bg(Color::Rgb(30 as u8, 30 as u8, 30 as u8));
        let footer_style = Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Gray);

        let groups_border = if self.focus == Focus::Groups {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let filter_border = if self.focus == Focus::Filter {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

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

        Line::from("Tab Switch pane  ↑↓ Move  Enter Edit/Run  Esc Cancel  q Quit")
            .style(footer_style)
            .render(footer[0], buf);

        Line::from("v0.1.0")
            .right_aligned()
            .style(footer_style)
            .render(footer[1], buf);

        let groups_block_style = if self.focus == Focus::Groups {
            Style::default().bg(Color::Black).fg(Color::White)
        } else {
            Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140))
        };

        let groups_block = Block::bordered()
            .title("Groups (Tab to switch)")
            .style(groups_block_style)
            .border_style(groups_border);

        let inner = groups_block.inner(groups_row[0]);
        groups_block.render(groups_row[0], buf);

        let filter_block_style = if self.focus == Focus::Filter {
            Style::default().bg(Color::Rgb(20, 20, 20)).fg(Color::White)
        } else {
            Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140))
        };

        let filter_block = Block::bordered()
            .title("Filter")
            .style(filter_block_style)
            .border_style(filter_border);

        let filter_inner = filter_block.inner(groups_row[1]);
        filter_block.render(groups_row[1], buf);

        let results_border = if self.focus == Focus::Results {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let results_block_style = if self.focus == Focus::Results {
            Style::default().bg(Color::Black).fg(Color::White)
        } else {
            Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140))
        };

        let results_block = Block::bordered()
            .title("Results")
            .style(results_block_style)
            .border_style(results_border);

        // results_block.render(chunks[2], buf);
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

            Line::from(msg)
                .style(Style::default().fg(Color::Gray))
                .render(
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

        let mut all_lines: Vec<&str> = Vec::new();
        for entry in &self.lines {
            for l in entry.lines() {
                all_lines.push(l);
            }
        }

        let scrollbar_w = 1u16;
        let text_area = Rect {
            x: results_inner.x,
            y: results_inner.y,
            width: results_inner.width.saturating_sub(scrollbar_w),
            height: results_inner.height,
        };

        let height = text_area.height as usize;
        let start = self.results_scroll;
        let end = (start + height).min(all_lines.len());

        for (i, line) in all_lines[start..end].iter().enumerate() {
            Line::from(*line).render(
                Rect {
                    x: text_area.x,
                    y: text_area.y + i as u16,
                    width: text_area.width,
                    height: 1,
                },
                buf,
            );
        }

        App::draw_scrollbar(
            buf,
            results_inner,
            self.results_scroll,
            all_lines.len(),
            self.focus == Focus::Results,
        );

        let mut row_y = filter_inner.y;

        let field_style =
            |field: FilterField, focus: Focus, current: FilterField, editing: bool| {
                if focus == Focus::Filter && field == current {
                    if editing {
                        // actively editing → strong highlight
                        Style::default().bg(Color::Gray).fg(Color::Black)
                    } else {
                        // focused but not editing → white
                        Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20))
                    }
                } else {
                    // unfocused field
                    Style::default()
                        .fg(Color::Rgb(100, 100, 100))
                        .bg(Color::Rgb(20, 20, 20))
                }
            };

        let line = |label: &str, value: &str| format!("{label}: {value}");

        Line::from(line("Start", &self.filter_start))
            .style(field_style(
                FilterField::Start,
                self.focus,
                self.filter_field,
                self.editing,
            ))
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
            .style(field_style(
                FilterField::End,
                self.focus,
                self.filter_field,
                self.editing,
            ))
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
            .style(field_style(
                FilterField::Query,
                self.focus,
                self.filter_field,
                self.editing,
            ))
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
            let field_row = match self.filter_field {
                FilterField::Start => 0,
                FilterField::End => 1,
                FilterField::Query => 2,
                FilterField::Search => 3, // no typing here; you can skip if you prefer
            };

            // Only show cursor for text fields
            if self.filter_field != FilterField::Search {
                let label = match self.filter_field {
                    FilterField::Start => "Start: ",
                    FilterField::End => "End: ",
                    FilterField::Query => "Query: ",
                    FilterField::Search => "",
                };

                let value_len = self.active_field_len(); // add helper below
                let y = filter_inner.y + field_row;

                // Cursor x = left + label width + typed text width
                let mut x = filter_inner.x + label.len() as u16 + value_len as u16;

                // clamp within the filter box
                let max_x = filter_inner.x + filter_inner.width.saturating_sub(1);
                if x > max_x {
                    x = max_x;
                }

                // draw a vertical bar cursor
                buf.get_mut(x, y)
                    .set_char('▏')
                    .set_style(Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20)));
            }
        }

        // "button"
        let btn = "[ Search ]";
        Line::from(btn)
            .style(field_style(
                FilterField::Search,
                self.focus,
                self.filter_field,
                false,
            ))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
    }
}
