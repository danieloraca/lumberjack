use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use ratatui::crossterm::event;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
use ratatui::{DefaultTerminal, Frame};

use crate::aws::fetch_log_events;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    Groups,
    Filter,
    Results,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FilterField {
    Start,
    End,
    Query,
    Search,
}

pub struct App {
    pub app_title: String,
    pub exit: bool,
    pub lines: Vec<String>,

    pub all_groups: Vec<String>,
    pub groups: Vec<String>,
    pub selected_group: usize,
    pub groups_scroll: usize,

    pub profile: String,
    pub region: String,
    pub focus: Focus,

    pub filter_start: String,
    pub filter_end: String,
    pub filter_query: String,
    pub filter_field: FilterField,
    pub editing: bool,
    pub cursor_on: bool,
    pub last_blink: Instant,

    pub group_search_active: bool,
    pub group_search_input: String,

    pub search_tx: Sender<String>,
    pub search_rx: Receiver<String>,
    pub searching: bool,
    pub dots: usize,
    pub last_dots: Instant,
    pub results_scroll: usize,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            if self.focus == Focus::Filter && self.editing {
                if self.last_blink.elapsed() >= Duration::from_millis(500) {
                    self.cursor_on = !self.cursor_on;
                    self.last_blink = Instant::now();
                }
            } else {
                self.cursor_on = true;
                self.last_blink = Instant::now();
            }

            while let Ok(msg) = self.search_rx.try_recv() {
                let total = self.results_total_lines();
                self.results_scroll = self.results_scroll.min(total.saturating_sub(1));

                if msg == "__SEARCH_DONE__" {
                    self.searching = false;
                    // when done, move focus to results so arrows can scroll later etc.
                    self.focus = Focus::Results;
                    continue;
                }

                self.lines.push(msg);
                // optional cap
                if self.lines.len() > 2000 {
                    self.lines.drain(0..500);
                }
            }

            if self.searching && self.last_dots.elapsed() >= Duration::from_millis(250) {
                self.dots = (self.dots + 1) % 7;
                self.last_dots = Instant::now();
            }

            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let event::Event::Key(key_event) = event::read()? {
                    self.handle_key_event(key_event)?;
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(&*self, frame.area());
    }

    fn handle_key_event(
        &mut self,
        key_event: ratatui::crossterm::event::KeyEvent,
    ) -> io::Result<()> {
        if key_event.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key_event.code {
            // q should NOT quit while editing or while group search is active
            KeyCode::Char('q') if !self.editing && !self.group_search_active => self.exit = true,

            // Switch focus between panes
            KeyCode::Tab if !self.editing => {
                self.focus = match self.focus {
                    Focus::Groups => Focus::Filter,
                    Focus::Filter => Focus::Groups,
                    Focus::Results => Focus::Groups,
                };
            }

            // Start group search
            KeyCode::Char('/') if self.focus == Focus::Groups && !self.editing => {
                self.group_search_active = true;
                self.group_search_input.clear();
                self.focus = Focus::Groups;
                return Ok(());
            }

            // ESC cancels group search or filter editing
            KeyCode::Esc => {
                if self.group_search_active {
                    self.group_search_active = false;
                    self.group_search_input.clear();
                    self.apply_group_search_filter();
                    return Ok(());
                }
                self.editing = false;
            }

            // While group search is active: handle its input
            KeyCode::Backspace if self.group_search_active => {
                self.group_search_input.pop();
                self.apply_group_search_filter();
                return Ok(());
            }
            KeyCode::Char(c) if self.group_search_active => {
                if !c.is_control() {
                    self.group_search_input.push(c);
                    self.apply_group_search_filter();
                }
                return Ok(());
            }

            // Confirm search with Enter: exit search mode but keep filtered groups
            KeyCode::Enter if self.group_search_active => {
                self.group_search_active = false;
                return Ok(());
            }

            // === Filter editing logic (restored) ===

            // While editing filter fields: route text input there
            KeyCode::Backspace if self.editing => {
                self.active_field_mut().pop();
            }
            KeyCode::Char(c) if self.editing => {
                self.active_field_mut().push(c);
            }

            // Enter: start/stop editing, or activate Search button
            KeyCode::Enter => {
                if self.focus == Focus::Filter
                    && self.filter_field == FilterField::Search
                    && !self.editing
                {
                    self.start_search();
                } else {
                    self.editing = !self.editing;
                }
            }

            // Navigation when NOT editing
            KeyCode::Up if !self.editing => match self.focus {
                Focus::Groups => self.groups_up(),
                Focus::Filter => self.filter_prev(),
                Focus::Results => self.results_up(),
            },

            KeyCode::Down if !self.editing => match self.focus {
                Focus::Groups => self.groups_down(),
                Focus::Filter => self.filter_next(),
                Focus::Results => self.results_down(),
            },

            _ => {}
        }

        Ok(())
    }

    pub fn draw_scrollbar(
        buf: &mut ratatui::buffer::Buffer,
        area: Rect,
        scroll: usize,
        total: usize,
        focus: bool,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // x column for scrollbar (rightmost column inside results block)
        let x = area.x + area.width - 1;

        // Style: subtle when unfocused, brighter when focused
        let track_style = if focus {
            Style::default()
                .fg(Color::Rgb(100, 100, 100))
                .bg(Color::Black)
        } else {
            Style::default()
                .fg(Color::Rgb(60, 60, 60))
                .bg(Color::Rgb(14, 14, 14))
        };

        let thumb_style = if focus {
            Style::default().fg(Color::White).bg(Color::Black)
        } else {
            Style::default()
                .fg(Color::Rgb(180, 180, 180))
                .bg(Color::Rgb(14, 14, 14))
        };

        // draw track
        for dy in 0..area.height {
            buf.get_mut(x, area.y + dy)
                .set_char('│')
                .set_style(track_style);
        }

        if total <= 1 {
            // full thumb
            buf.get_mut(x, area.y).set_char('█').set_style(thumb_style);
            return;
        }

        let view = area.height as usize;
        if view == 0 {
            return;
        }

        // thumb size at least 1
        let thumb_h = ((view * view) / total).clamp(1, view);
        let max_scroll = total.saturating_sub(view);
        let scroll = scroll.min(max_scroll);

        // thumb position
        let thumb_top = if max_scroll == 0 {
            0
        } else {
            (scroll * (view - thumb_h)) / max_scroll
        };

        for i in 0..thumb_h {
            let y = area.y + (thumb_top + i) as u16;
            buf.get_mut(x, y).set_char('█').set_style(thumb_style);
        }
    }

    fn results_up(&mut self) {
        self.results_scroll = self.results_scroll.saturating_sub(1);
    }

    fn results_total_lines(&self) -> usize {
        self.lines.iter().map(|s| s.lines().count()).sum()
    }

    fn results_down(&mut self) {
        let total = self.results_total_lines();
        if self.results_scroll + 1 < total {
            self.results_scroll += 1;
        }
    }

    fn visible_group_rows(&self) -> usize {
        4
    }

    fn clamp_groups_scroll(&mut self, visible_rows: usize) {
        if self.groups.is_empty() {
            self.groups_scroll = 0;
            self.selected_group = 0;
            return;
        }

        if self.selected_group < self.groups_scroll {
            self.groups_scroll = self.selected_group;
        } else if self.selected_group >= self.groups_scroll + visible_rows {
            self.groups_scroll = self.selected_group + 1 - visible_rows;
        }

        let max_scroll = self.groups.len().saturating_sub(visible_rows);
        self.groups_scroll = self.groups_scroll.min(max_scroll);
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.filter_field {
            FilterField::Start => &mut self.filter_start,
            FilterField::End => &mut self.filter_end,
            FilterField::Query => &mut self.filter_query,
            FilterField::Search => &mut self.filter_query, // unused; won't type into Search
        }
    }

    fn groups_up(&mut self) {
        if !self.groups.is_empty() {
            self.selected_group = self.selected_group.saturating_sub(1);
            self.clamp_groups_scroll(self.visible_group_rows());
        }
    }
    fn groups_down(&mut self) {
        if !self.groups.is_empty() {
            self.selected_group = (self.selected_group + 1).min(self.groups.len() - 1);
            self.clamp_groups_scroll(self.visible_group_rows());
        }
    }

    fn filter_prev(&mut self) {
        self.filter_field = match self.filter_field {
            FilterField::Start => FilterField::Start,
            FilterField::End => FilterField::Start,
            FilterField::Query => FilterField::End,
            FilterField::Search => FilterField::Query,
        };
    }
    fn filter_next(&mut self) {
        self.filter_field = match self.filter_field {
            FilterField::Start => FilterField::End,
            FilterField::End => FilterField::Query,
            FilterField::Query => FilterField::Search,
            FilterField::Search => FilterField::Search,
        };
    }

    fn start_search(&mut self) {
        self.searching = true;
        self.dots = 0;
        self.last_dots = Instant::now();
        self.focus = Focus::Results; // lose focus from form
        self.editing = false;
        self.lines.clear(); // optional
        self.results_scroll = 0;

        let group = match self.groups.get(self.selected_group) {
            Some(g) => g.clone(),
            None => return,
        };

        let region = self.region.clone();
        let profile = self.profile.clone();
        let start = self.filter_start.clone();
        let end = self.filter_end.clone();
        let pattern = self.filter_query.clone();

        let tx = self.search_tx.clone();

        // show immediate feedback
        let _ = tx.send(format!("Searching {} ...", group));

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            let res = rt.block_on(fetch_log_events(
                &region,
                &profile,
                &group,
                start.as_str(),
                end.as_str(),
                pattern.as_str(),
            ));

            match res {
                Ok(lines) => {
                    let _ = tx.send(format!("--- {} results ---", lines.len()));
                    for line in lines {
                        let _ = tx.send(line);
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("[search error] {e}"));
                }
            }

            let _ = tx.send("__SEARCH_DONE__".to_string());
        });
    }

    pub fn active_field_len(&self) -> usize {
        match self.filter_field {
            FilterField::Start => self.filter_start.len(),
            FilterField::End => self.filter_end.len(),
            FilterField::Query => self.filter_query.len(),
            FilterField::Search => 0,
        }
    }

    fn fuzzy_match(haystack: &str, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }

        let haystack = haystack.to_lowercase();
        let needle = needle.to_lowercase();
        let mut it = haystack.chars();

        for c in needle.chars() {
            if let Some(_) = it.by_ref().find(|&hc| hc == c) {
                continue;
            } else {
                return false;
            }
        }
        true
    }

    fn apply_group_search_filter(&mut self) {
        if !self.group_search_active || self.group_search_input.is_empty() {
            // No active search → restore original list
            self.groups = self.all_groups.clone();
            self.selected_group = 0;
            self.groups_scroll = 0;
            return;
        }

        let pattern = self.group_search_input.clone();
        let mut filtered: Vec<String> = self
            .all_groups
            .iter()
            .filter(|g| Self::fuzzy_match(g, &pattern))
            .cloned()
            .collect();

        if filtered.is_empty() {
            filtered.push("(no matches)".to_string());
        }

        self.groups = filtered;
        self.selected_group = 0;
        self.groups_scroll = 0;
    }
}
