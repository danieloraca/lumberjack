mod clipboard;
mod filters;
mod keymap;

use std::io;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use chrono::Utc;
use ratatui::crossterm::event;
use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
use ratatui::{DefaultTerminal, Frame};

use crate::aws::fetch_log_events;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedFilter {
    pub name: String,
    #[serde(default)]
    pub group: String,
    pub start: String,
    pub end: String,
    pub query: String,
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

    pub tail_mode: bool,
    pub tail_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,

    pub status_message: Option<String>,
    pub status_set_at: Option<Instant>,

    pub saved_filters: Vec<SavedFilter>,

    pub save_filter_popup_open: bool,
    pub save_filter_name: String,

    pub load_filter_popup_open: bool,
    pub load_filter_selected: usize,
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

            // Clear transient status messages after 2 seconds
            self.maybe_clear_status();

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
            if let Some(cell) = buf.cell_mut((x, area.y + dy)) {
                cell.set_char('│').set_style(track_style);
            }
        }

        if total <= 1 {
            // full thumb
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char('█').set_style(thumb_style);
            }

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
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char('█').set_style(thumb_style);
            }
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
        // Up arrow: move backward and wrap
        self.filter_field = match self.filter_field {
            FilterField::Start => FilterField::Search,
            FilterField::End => FilterField::Start,
            FilterField::Query => FilterField::End,
            FilterField::Search => FilterField::Query,
        };
    }

    fn filter_next(&mut self) {
        // Down arrow: move forward and wrap
        self.filter_field = match self.filter_field {
            FilterField::Start => FilterField::End,
            FilterField::End => FilterField::Query,
            FilterField::Query => FilterField::Search,
            FilterField::Search => FilterField::Start,
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
        self.tail_stop.store(false, Ordering::Relaxed);

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

        let tail_mode = self.tail_mode;
        let tail_stop = self.tail_stop.clone();

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

            let mut last_ts: Option<i64> = None;

            match res {
                Ok((lines, last)) => {
                    let _ = tx.send(format!("--- {} results ---", lines.len()));
                    for line in lines {
                        let _ = tx.send(line);
                    }
                    last_ts = last;
                }
                Err(e) => {
                    let _ = tx.send(format!("[search error] {e}"));
                }
            }

            // If not tailing, we're done
            if !tail_mode {
                let _ = tx.send("__SEARCH_DONE__".to_string());
                return;
            }

            // Tail mode: repeatedly fetch new events
            loop {
                if tail_stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // Build new time window: from last_ts+1 (or start) to now
                let tail_start = if let Some(ts) = last_ts {
                    if let Some(dt) = chrono::DateTime::<Utc>::from_timestamp_millis(ts + 1) {
                        dt.to_rfc3339()
                    } else {
                        start.clone() // fallback
                    }
                } else {
                    start.clone()
                };

                // Empty end = "now" (fetch_log_events treats empty end as now)
                let tail_end = String::new();

                let res = rt.block_on(fetch_log_events(
                    &region,
                    &profile,
                    &group,
                    tail_start.as_str(),
                    tail_end.as_str(),
                    pattern.as_str(),
                ));

                match res {
                    Ok((lines, new_last)) => {
                        // Don’t re-print a header every poll; just append lines
                        for line in lines {
                            let _ = tx.send(line);
                        }
                        if let Some(ts) = new_last {
                            last_ts = Some(last_ts.map_or(ts, |prev| prev.max(ts)));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("[tail error] {e}"));
                        // optional: break on repeated errors
                    }
                }

                // Simple tail interval
                std::thread::sleep(std::time::Duration::from_secs(3));
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

    fn apply_time_preset(&mut self, start: &str) {
        self.filter_start = start.to_string();
        self.filter_end.clear(); // empty = "now"

        self.filter_field = FilterField::Query;

        // Ensure we're not in editing mode
        self.editing = false;
    }

    fn maybe_clear_status(&mut self) {
        if let Some(set_at) = self.status_set_at {
            if set_at.elapsed() >= Duration::from_secs(2) {
                self.status_message = None;
                self.status_set_at = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_groups(groups: Vec<&str>) -> App {
        let groups_owned: Vec<String> = groups.iter().map(|s| s.to_string()).collect();
        let (tx, rx) = std::sync::mpsc::channel();

        App {
            app_title: "Test".to_string(),
            exit: false,
            lines: Vec::new(),

            all_groups: groups_owned.clone(),
            groups: groups_owned,
            selected_group: 0,
            groups_scroll: 0,

            profile: "test-profile".to_string(),
            region: "eu-west-1".to_string(),
            focus: Focus::Groups,

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

            saved_filters: Vec::new(),
            save_filter_popup_open: false,
            save_filter_name: String::new(),
            load_filter_popup_open: false,
            load_filter_selected: 0,
        }
    }

    // --- fuzzy_match tests ---

    #[test]
    fn fuzzy_match_empty_needle_matches_anything() {
        assert!(App::fuzzy_match("hello", ""));
        assert!(App::fuzzy_match("", ""));
    }

    #[test]
    fn fuzzy_match_simple_subsequence() {
        assert!(App::fuzzy_match("aws-lambda-api", "ala")); // a-l-a in order
        assert!(App::fuzzy_match("/aws/lambda/foo", "alf")); // a-l-f in order
        assert!(!App::fuzzy_match("cloudwatch", "cwz")); // z not present
    }

    #[test]
    fn fuzzy_match_case_insensitive() {
        assert!(App::fuzzy_match("AWS-LAMBDA", "aws"));
        assert!(App::fuzzy_match("aws-lambda", "AWS"));
    }

    // --- apply_group_search_filter tests ---

    #[test]
    fn apply_group_search_filter_restores_full_list_when_inactive() {
        let mut app = app_with_groups(vec!["/aws/lambda/api", "/aws/lambda/worker"]);

        // not active, even if input is non-empty → should ignore and restore full list
        app.group_search_active = false;
        app.group_search_input = "api".to_string();
        app.apply_group_search_filter();

        assert_eq!(app.groups.len(), 2);
        assert_eq!(app.groups[0], "/aws/lambda/api");
        assert_eq!(app.groups[1], "/aws/lambda/worker");
    }

    #[test]
    fn apply_group_search_filter_filters_when_active() {
        let mut app = app_with_groups(vec!["/aws/lambda/api", "/aws/lambda/worker"]);

        app.group_search_active = true;
        app.group_search_input = "wrk".to_string(); // matches "worker"

        app.apply_group_search_filter();

        assert_eq!(app.groups.len(), 1);
        assert_eq!(app.groups[0], "/aws/lambda/worker");
        assert_eq!(app.selected_group, 0);
        assert_eq!(app.groups_scroll, 0);
    }

    #[test]
    fn apply_group_search_filter_no_matches_shows_placeholder() {
        let mut app = app_with_groups(vec!["/aws/lambda/api", "/aws/lambda/worker"]);

        app.group_search_active = true;
        app.group_search_input = "xyz".to_string();

        app.apply_group_search_filter();

        assert_eq!(app.groups.len(), 1);
        assert_eq!(app.groups[0], "(no matches)");
        assert_eq!(app.selected_group, 0);
    }

    #[test]
    fn apply_group_search_filter_clearing_input_restores_all_groups() {
        let mut app = app_with_groups(vec!["/aws/lambda/api", "/aws/lambda/worker"]);

        // First, narrow to one
        app.group_search_active = true;
        app.group_search_input = "api".to_string();
        app.apply_group_search_filter();
        assert_eq!(app.groups.len(), 1);

        // Then, clear the input and reapply
        app.group_search_input.clear();
        app.apply_group_search_filter();

        assert_eq!(app.groups.len(), 2);
        assert_eq!(app.groups[0], "/aws/lambda/api");
        assert_eq!(app.groups[1], "/aws/lambda/worker");
    }

    #[test]
    fn apply_time_preset_sets_start_and_clears_end() {
        let mut app = app_with_groups(vec!["/aws/lambda/api"]);

        app.filter_start = "2025-12-11T10:00:00Z".to_string();
        app.filter_end = "2025-12-11T11:00:00Z".to_string();
        app.filter_field = FilterField::Start;
        app.editing = true;

        app.apply_time_preset("-15m");

        assert_eq!(app.filter_start, "-15m");
        assert_eq!(app.filter_end, "");
        assert_eq!(app.filter_field, FilterField::Query);
        assert!(!app.editing);
    }

    #[test]
    fn maybe_clear_status_clears_after_timeout() {
        let mut app = app_with_groups(vec!["/aws/lambda/api"]);
        app.status_message = Some("test".to_string());

        // Simulate a status set in the past by manually setting status_set_at
        // to an Instant that is guaranteed to have "elapsed" >= 2s.
        app.status_set_at = Some(Instant::now() - Duration::from_secs(3));

        app.maybe_clear_status();

        assert!(
            app.status_message.is_none(),
            "expected status_message to be cleared after timeout"
        );
        assert!(
            app.status_set_at.is_none(),
            "expected status_set_at to be cleared after timeout"
        );
    }
}
