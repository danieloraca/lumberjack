use std::io;

use ratatui::crossterm::event::{KeyCode, KeyEventKind};

use super::{App, FilterField, Focus};

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

            _ => {}
        }

        Ok(())
    }
}
