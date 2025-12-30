use super::{App, FilterField, Focus};
use crate::ui::styles::Theme;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use std::io;

impl App {
    pub fn handle_key_event(
        &mut self,
        key_event: ratatui::crossterm::event::KeyEvent,
    ) -> io::Result<()> {
        if key_event.kind != KeyEventKind::Press {
            return Ok(());
        }

        // While a popup is open, route keys to the popup handlers first.
        if self.save_filter_popup_open {
            self.handle_save_filter_popup_key(key_event.code);
            return Ok(());
        }
        if self.load_filter_popup_open {
            self.handle_load_filter_popup_key(key_event.code);
            return Ok(());
        }

        match key_event.code {
            // q should NOT quit while editing or while group search is active
            KeyCode::Char('q') if !self.editing && !self.group_search_active => {
                self.tail_stop
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.exit = true;
            }

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

            // === Filter editing logic ===
            // Move cursor within the active field
            KeyCode::Left if self.editing => {
                if self.filter_cursor_pos > 0 {
                    self.filter_cursor_pos -= 1;
                }
            }
            KeyCode::Right if self.editing => {
                let len = self.active_field_len();
                if self.filter_cursor_pos < len {
                    self.filter_cursor_pos += 1;
                }
            }

            // Delete char before cursor (Backspace)
            KeyCode::Backspace if self.editing => {
                let len = self.active_field_len();
                if self.filter_cursor_pos > 0 && len > 0 {
                    let idx = self.filter_cursor_pos;
                    let field = self.active_field_mut();
                    // Work on bytes; fine for ASCII queries
                    if idx <= field.len() {
                        field.remove(idx - 1);
                        self.filter_cursor_pos -= 1;
                    }
                }
            }

            // Insert char at cursor
            KeyCode::Char(c) if self.editing => {
                if !c.is_control() {
                    let idx = self.filter_cursor_pos;
                    let field = self.active_field_mut();
                    if idx <= field.len() {
                        field.insert(idx, c);
                        self.filter_cursor_pos += 1;
                    }
                }
            }

            // Enter: start/stop editing, or activate Search button
            KeyCode::Enter => {
                if self.focus == Focus::Filter
                    && self.filter_field == FilterField::Search
                    && !self.editing
                {
                    self.start_search();
                } else {
                    if !self.editing {
                        // entering edit mode: cursor at end of active field
                        self.filter_cursor_pos = self.active_field_len();
                    }
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

            // Copy results to clipboard (Results pane, not editing)
            KeyCode::Char('y') if !self.editing && self.focus == Focus::Results => {
                self.copy_results_to_clipboard();
            }

            // Toggle tail mode
            KeyCode::Char('t') if !self.editing && !self.group_search_active => {
                self.tail_mode = !self.tail_mode;
                if !self.tail_mode {
                    self.tail_stop
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }

            // Open "Save filter" popup (Filter pane, not editing)
            KeyCode::Char('s')
                if self.focus == Focus::Filter && !self.editing && !self.group_search_active =>
            {
                self.open_save_filter_popup();
            }

            // Open "Load filter" popup (any focus, not editing)
            KeyCode::Char('F') if !self.editing && !self.group_search_active => {
                self.open_load_filter_popup();
            }

            // Quick time presets (Filter pane, not editing)
            KeyCode::Char('1')
                if self.focus == Focus::Filter && !self.editing && !self.group_search_active =>
            {
                self.apply_time_preset("-5m");
            }
            KeyCode::Char('2')
                if self.focus == Focus::Filter && !self.editing && !self.group_search_active =>
            {
                self.apply_time_preset("-15m");
            }
            KeyCode::Char('3')
                if self.focus == Focus::Filter && !self.editing && !self.group_search_active =>
            {
                self.apply_time_preset("-1h");
            }
            KeyCode::Char('4')
                if self.focus == Focus::Filter && !self.editing && !self.group_search_active =>
            {
                self.apply_time_preset("-24m");
            }

            KeyCode::Char('T') if !self.editing => {
                if self.theme_name == "dark" {
                    self.theme = Theme::light();
                    self.theme_name = "light".to_string();
                } else if self.theme_name == "light" {
                    self.theme = Theme::green();
                    self.theme_name = "green".to_string();
                } else {
                    self.theme = Theme::default_dark();
                    self.theme_name = "dark".to_string();
                }
            }

            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, FilterField, Focus};
    use crate::ui::styles::Theme;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::sync::mpsc;
    use std::time::Instant as StdInstant;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with_filter_query(query: &str) -> App {
        let (tx, rx) = mpsc::channel();

        App {
            app_title: "Test".to_string(),
            theme: Theme::default_dark(),
            theme_name: "dark".to_string(),
            exit: false,
            lines: Vec::new(),
            filter_cursor_pos: 0,

            all_groups: Vec::new(),
            groups: Vec::new(),
            selected_group: 0,
            groups_scroll: 0,

            profile: "test-profile".to_string(),
            region: "eu-west-1".to_string(),
            focus: Focus::Filter,

            filter_start: String::new(),
            filter_end: String::new(),
            filter_query: query.to_string(),
            filter_field: FilterField::Query,
            editing: false,
            cursor_on: true,
            last_blink: StdInstant::now(),

            group_search_active: false,
            group_search_input: String::new(),

            search_tx: tx,
            search_rx: rx,
            searching: false,
            dots: 0,
            last_dots: StdInstant::now(),
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

    #[test]
    fn cursor_moves_and_inserts_in_middle_of_query() {
        let mut app = app_with_filter_query("abc");
        app.focus = Focus::Filter;

        // Enter edit mode on Query
        app.handle_key_event(key(KeyCode::Enter)).unwrap();
        assert!(app.editing);
        assert_eq!(app.filter_cursor_pos, 3); // at end of "abc"

        // Move cursor left once: position between 'b' and 'c'
        app.handle_key_event(key(KeyCode::Left)).unwrap();
        assert_eq!(app.filter_cursor_pos, 2);

        // Insert 'X' at position 2: "abXc"
        app.handle_key_event(key(KeyCode::Char('X'))).unwrap();
        assert_eq!(app.filter_query, "abXc");
        assert_eq!(app.filter_cursor_pos, 3); // now after 'X'

        // Backspace: delete 'X', back to "abc"
        app.handle_key_event(key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.filter_query, "abc");
        assert_eq!(app.filter_cursor_pos, 2); // back between 'b' and 'c'
    }
}
