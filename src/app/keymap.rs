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
        if self.state.save_filter_popup_open {
            self.handle_save_filter_popup_key(key_event.code);
            return Ok(());
        }
        if self.state.load_filter_popup_open {
            self.handle_load_filter_popup_key(key_event.code);
            return Ok(());
        }

        match key_event.code {
            // q should NOT quit while editing or while group search is active
            KeyCode::Char('q') if !self.state.editing && !self.state.group_search_active => {
                self.tail_stop
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.exit = true;
            }

            // Switch focus between panes
            KeyCode::Tab if !self.state.editing => {
                self.state.focus = match self.state.focus {
                    Focus::Groups => Focus::Filter,
                    Focus::Filter => Focus::Groups,
                    Focus::Results => Focus::Groups,
                };
            }

            // Start group search
            KeyCode::Char('/') if self.state.focus == Focus::Groups && !self.state.editing => {
                self.state.group_search_active = true;
                self.state.focus = Focus::Groups;
                return Ok(());
            }

            // ESC cancels group search or filter editing
            KeyCode::Esc => {
                if self.state.group_search_active {
                    self.state.group_search_active = false;
                    self.state.group_search_input.clear();
                    self.apply_group_search_filter();
                    return Ok(());
                }
                self.state.editing = false;
            }

            // While group search is active: handle its input
            KeyCode::Backspace if self.state.group_search_active => {
                self.state.group_search_input.pop();
                self.apply_group_search_filter();
                return Ok(());
            }
            KeyCode::Char(c) if self.state.group_search_active => {
                if !c.is_control() {
                    self.state.group_search_input.push(c);
                    self.apply_group_search_filter();
                }
                return Ok(());
            }

            // Confirm search with Enter: exit search mode but keep filtered groups
            KeyCode::Enter if self.state.group_search_active => {
                self.state.group_search_active = false;
                return Ok(());
            }

            // === Filter editing logic ===
            // Move cursor within the active field
            KeyCode::Left if self.state.editing => {
                if self.state.filter_cursor_pos > 0 {
                    self.state.filter_cursor_pos -= 1;
                }
            }
            KeyCode::Right if self.state.editing => {
                let len = self.active_field_len();
                if self.state.filter_cursor_pos < len {
                    self.state.filter_cursor_pos += 1;
                }
            }

            // Delete char before cursor (Backspace)
            KeyCode::Backspace if self.state.editing => {
                let len = self.active_field_len();
                if self.state.filter_cursor_pos > 0 && len > 0 {
                    let idx = self.state.filter_cursor_pos;
                    let field = self.active_field_mut();
                    // Work on bytes; fine for ASCII queries
                    if idx <= field.len() {
                        field.remove(idx - 1);
                        self.state.filter_cursor_pos -= 1;
                    }
                }
            }

            // Insert char at cursor
            KeyCode::Char(c) if self.state.editing => {
                if !c.is_control() {
                    let idx = self.state.filter_cursor_pos;
                    let field = self.active_field_mut();
                    if idx <= field.len() {
                        field.insert(idx, c);
                        self.state.filter_cursor_pos += 1;
                    }
                }
            }

            // Enter: start/stop editing, or activate Search button
            KeyCode::Enter => {
                if self.state.focus == Focus::Filter
                    && self.state.filter_field == FilterField::Search
                    && !self.state.editing
                {
                    self.start_search();
                } else {
                    if !self.state.editing {
                        // entering edit mode: cursor at end of active field
                        self.state.filter_cursor_pos = self.active_field_len();
                    }
                    self.state.editing = !self.state.editing;
                }
            }

            // Navigation when NOT editing
            KeyCode::Up if !self.state.editing => match self.state.focus {
                Focus::Groups => self.groups_up(),
                Focus::Filter => self.filter_prev(),
                Focus::Results => self.results_up(),
            },
            KeyCode::Down if !self.state.editing => match self.state.focus {
                Focus::Groups => self.groups_down(),
                Focus::Filter => self.filter_next(),
                Focus::Results => self.results_down(),
            },

            // Copy results to clipboard (Results pane, not editing)
            KeyCode::Char('y') if !self.state.editing && self.state.focus == Focus::Results => {
                self.copy_results_to_clipboard();
            }

            // Toggle tail mode
            KeyCode::Char('t') if !self.state.editing && !self.state.group_search_active => {
                self.state.tail_mode = !self.state.tail_mode;
                if !self.state.tail_mode {
                    self.tail_stop
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }

            // Open "Save filter" popup (Filter pane, not editing)
            KeyCode::Char('s')
                if self.state.focus == Focus::Filter
                    && !self.state.editing
                    && !self.state.group_search_active =>
            {
                self.open_save_filter_popup();
            }

            // Open "Load filter" popup (any focus, not editing)
            KeyCode::Char('F') if !self.state.editing && !self.state.group_search_active => {
                self.open_load_filter_popup();
            }

            // Quick time presets (Filter pane, not editing)
            KeyCode::Char('1')
                if self.state.focus == Focus::Filter
                    && !self.state.editing
                    && !self.state.group_search_active =>
            {
                self.apply_time_preset("-5m");
            }
            KeyCode::Char('2')
                if self.state.focus == Focus::Filter
                    && !self.state.editing
                    && !self.state.group_search_active =>
            {
                self.apply_time_preset("-15m");
            }
            KeyCode::Char('3')
                if self.state.focus == Focus::Filter
                    && !self.state.editing
                    && !self.state.group_search_active =>
            {
                self.apply_time_preset("-1h");
            }
            KeyCode::Char('4')
                if self.state.focus == Focus::Filter
                    && !self.state.editing
                    && !self.state.group_search_active =>
            {
                self.apply_time_preset("-24m");
            }

            KeyCode::Char('T') if !self.state.editing => {
                if self.state.theme_name == "dark" {
                    self.state.theme = Theme::light();
                    self.state.theme_name = "light".to_string();
                } else if self.state.theme_name == "light" {
                    self.state.theme = Theme::green();
                    self.state.theme_name = "green".to_string();
                } else {
                    self.state.theme = Theme::default_dark();
                    self.state.theme_name = "dark".to_string();
                }
            }

            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::AppState;
    use crate::app::{App, FilterField, Focus};
    use crate::ui::styles::Theme;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::sync::atomic::AtomicBool;

    use std::sync::{Arc, mpsc};
    use std::time::Instant as StdInstant;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with_filter_query(query: &str) -> App {
        let (tx, rx) = mpsc::channel();

        let state = AppState {
            app_title: "Test".to_string(),
            theme: Theme::default_dark(),
            theme_name: "dark".to_string(),
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

            searching: false,
            dots: 0,
            last_dots: StdInstant::now(),
            results_scroll: 0,

            tail_mode: false,

            status_message: None,
            status_set_at: None,

            saved_filters: Vec::new(),
            save_filter_popup_open: false,
            save_filter_name: String::new(),
            load_filter_popup_open: false,
            load_filter_selected: 0,
        };

        App {
            state,
            exit: false,
            search_tx: tx,
            search_rx: rx,
            tail_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn cursor_moves_and_inserts_in_middle_of_query() {
        let mut app = app_with_filter_query("abc");
        app.state.focus = Focus::Filter;

        // Enter edit mode on Query
        app.handle_key_event(key(KeyCode::Enter)).unwrap();
        assert!(app.state.editing);
        assert_eq!(app.state.filter_cursor_pos, 3); // at end of "abc"

        // Move cursor left once: position between 'b' and 'c'
        app.handle_key_event(key(KeyCode::Left)).unwrap();
        assert_eq!(app.state.filter_cursor_pos, 2);

        // Insert 'X' at position 2: "abXc"
        app.handle_key_event(key(KeyCode::Char('X'))).unwrap();
        assert_eq!(app.state.filter_query, "abXc");
        assert_eq!(app.state.filter_cursor_pos, 3); // now after 'X'

        // Backspace: delete 'X', back to "abc"
        app.handle_key_event(key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.state.filter_query, "abc");
        assert_eq!(app.state.filter_cursor_pos, 2); // back between 'b' and 'c'
    }

    #[test]
    fn theme_cycles_dark_light_green_on_t() {
        let mut app = app_with_filter_query("");
        // Ensure starting theme is dark
        assert_eq!(app.state.theme_name, "dark");

        // First T: dark -> light
        app.handle_key_event(key(KeyCode::Char('T'))).unwrap();
        assert_eq!(app.state.theme_name, "light");

        // Second T: light -> green
        app.handle_key_event(key(KeyCode::Char('T'))).unwrap();
        assert_eq!(app.state.theme_name, "green");

        // Third T: green -> dark
        app.handle_key_event(key(KeyCode::Char('T'))).unwrap();
        assert_eq!(app.state.theme_name, "dark");
    }
}
