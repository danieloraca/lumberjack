use std::time::Instant;

use ratatui::crossterm::event::KeyCode;

use super::{App, FilterField, SavedFilter};

impl App {
    pub fn open_save_filter_popup(&mut self) {
        self.save_filter_name.clear();
        self.save_filter_popup_open = true;
    }

    pub fn open_load_filter_popup(&mut self) {
        if self.saved_filters.is_empty() {
            self.status_message = Some("No saved filters".to_string());
            self.status_set_at = Some(Instant::now());
            return;
        }

        self.load_filter_selected = 0;
        self.load_filter_popup_open = true;
    }

    pub fn handle_save_filter_popup_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.save_filter_popup_open = false;
            }
            KeyCode::Enter => {
                if !self.save_filter_name.trim().is_empty() {
                    let name = self.save_filter_name.trim().to_string();
                    // Overwrite if exists
                    if let Some(existing) = self.saved_filters.iter_mut().find(|f| f.name == name) {
                        existing.start = self.filter_start.clone();
                        existing.end = self.filter_end.clone();
                        existing.query = self.filter_query.clone();
                    } else {
                        self.saved_filters.push(SavedFilter {
                            name: name.clone(),
                            start: self.filter_start.clone(),
                            end: self.filter_end.clone(),
                            query: self.filter_query.clone(),
                        });
                    }
                    self.status_message = Some(format!("Saved filter \"{}\"", name));
                    self.status_set_at = Some(Instant::now());
                }
                self.save_filter_popup_open = false;
            }
            KeyCode::Backspace => {
                self.save_filter_name.pop();
            }
            KeyCode::Char(c) => {
                if !c.is_control() {
                    self.save_filter_name.push(c);
                }
            }
            _ => {}
        }
    }

    pub fn handle_load_filter_popup_key(&mut self, code: KeyCode) {
        if self.saved_filters.is_empty() {
            self.load_filter_popup_open = false;
            return;
        }

        match code {
            KeyCode::Esc => {
                self.load_filter_popup_open = false;
            }
            KeyCode::Up => {
                if self.load_filter_selected > 0 {
                    self.load_filter_selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.load_filter_selected + 1 < self.saved_filters.len() {
                    self.load_filter_selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(f) = self.saved_filters.get(self.load_filter_selected) {
                    self.filter_start = f.start.clone();
                    self.filter_end = f.end.clone();
                    self.filter_query = f.query.clone();
                    self.filter_field = FilterField::Query;
                    self.status_message = Some(format!("Loaded filter \"{}\"", f.name));
                    self.status_set_at = Some(Instant::now());
                }
                self.load_filter_popup_open = false;
            }
            _ => {}
        }
    }
}
