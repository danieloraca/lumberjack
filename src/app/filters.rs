use std::path::PathBuf;
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
            // Try to load from disk lazily
            if let Ok(filters) = Self::load_saved_filters_from_disk() {
                self.saved_filters = filters;
            }
        }

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
                    let current_group = self
                        .groups
                        .get(self.selected_group)
                        .cloned()
                        .unwrap_or_default();
                    // Overwrite if exists
                    if let Some(existing) = self.saved_filters.iter_mut().find(|f| f.name == name) {
                        existing.group = current_group.clone();
                        existing.start = self.filter_start.clone();
                        existing.end = self.filter_end.clone();
                        existing.query = self.filter_query.clone();
                    } else {
                        self.saved_filters.push(SavedFilter {
                            name: name.clone(),
                            group: current_group.clone(),
                            start: self.filter_start.clone(),
                            end: self.filter_end.clone(),
                            query: self.filter_query.clone(),
                        });
                    }

                    // Best-effort persistence; update status on success or failure
                    match Self::save_all_filters_to_disk(&self.saved_filters) {
                        Ok(()) => {
                            self.status_message = Some(format!("Saved filter \"{}\"", name));
                        }
                        Err(e) => {
                            self.status_message =
                                Some(format!("Error saving filter \"{}\": {}", name, e));
                        }
                    }
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
                    // Try to select the saved group if it still exists in the groups list
                    if !f.group.is_empty() {
                        if let Some(idx) = self.groups.iter().position(|g| g == &f.group) {
                            self.selected_group = idx;
                            self.groups_scroll = 0; // or clamp via clamp_groups_scroll later
                        }
                    }
                    self.status_message = Some(format!("Loaded filter \"{}\"", f.name));
                    self.status_set_at = Some(Instant::now());
                }
                self.load_filter_popup_open = false;
            }
            _ => {}
        }
    }

    fn filters_path() -> Result<PathBuf, String> {
        // In tests, write filters to a separate location so we don't overwrite
        // the user's real filters.
        if cfg!(test) {
            let home = std::env::var("HOME").map_err(|e| format!("HOME not set: {e}"))?;
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("lumberjack-test");
            std::fs::create_dir_all(&path)
                .map_err(|e| format!("create_dir_all {}: {e}", path.display()))?;
            path.push("filters.json");
            return Ok(path);
        }

        // Normal runtime path
        let home = std::env::var("HOME").map_err(|e| format!("HOME not set: {e}"))?;
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("lumberjack");
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("create_dir_all {}: {e}", path.display()))?;
        path.push("filters.json");
        Ok(path)
    }

    fn load_saved_filters_from_disk() -> Result<Vec<SavedFilter>, String> {
        let path = Self::filters_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("read_to_string {}: {e}", path.display()))?;
        let filters: Vec<SavedFilter> =
            serde_json::from_str(&data).map_err(|e| format!("decode: {e}"))?;
        Ok(filters)
    }

    fn save_all_filters_to_disk(filters: &[SavedFilter]) -> Result<(), String> {
        let path = Self::filters_path()?;
        let data = serde_json::to_string_pretty(filters).map_err(|e| format!("encode: {e}"))?;
        std::fs::write(&path, data).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Focus};
    use std::sync::mpsc;
    use std::time::Instant as StdInstant;

    fn app_with_filter_state() -> App {
        let (tx, rx) = mpsc::channel();

        App {
            app_title: "Test".to_string(),
            exit: false,
            lines: Vec::new(),

            all_groups: Vec::new(),
            groups: Vec::new(),
            selected_group: 0,
            groups_scroll: 0,

            profile: "test-profile".to_string(),
            region: "eu-west-1".to_string(),
            focus: Focus::Filter,

            filter_start: String::new(),
            filter_end: String::new(),
            filter_query: String::new(),
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
    fn save_filter_creates_new_entry() {
        let mut app = app_with_filter_state();
        app.filter_start = "-5m".to_string();
        app.filter_end = "".to_string();
        app.filter_query = "routing_id=123".to_string();

        app.open_save_filter_popup();
        app.save_filter_name = "quick-errors".to_string();
        app.handle_save_filter_popup_key(KeyCode::Enter);

        assert_eq!(app.saved_filters.len(), 1);
        let f = &app.saved_filters[0];
        assert_eq!(f.name, "quick-errors");
        assert_eq!(f.start, "-5m");
        assert_eq!(f.end, "");
        assert_eq!(f.query, "routing_id=123");
    }

    #[test]
    fn save_filter_overwrites_existing_entry_with_same_name() {
        let mut app = app_with_filter_state();
        app.saved_filters.push(SavedFilter {
            name: "quick-errors".to_string(),
            group: "".to_string(),
            start: "-5m".to_string(),
            end: "".to_string(),
            query: "routing_id=111".to_string(),
        });

        app.filter_start = "-15m".to_string();
        app.filter_end = "".to_string();
        app.filter_query = "routing_id=222".to_string();

        app.open_save_filter_popup();
        app.save_filter_name = "quick-errors".to_string();
        app.handle_save_filter_popup_key(KeyCode::Enter);

        assert_eq!(app.saved_filters.len(), 1);
        let f = &app.saved_filters[0];
        assert_eq!(f.name, "quick-errors");
        assert_eq!(f.start, "-15m");
        assert_eq!(f.end, "");
        assert_eq!(f.query, "routing_id=222");
    }

    #[test]
    fn load_filter_applies_selected_values_to_filter_fields() {
        let mut app = app_with_filter_state();
        app.saved_filters.push(SavedFilter {
            name: "last-hour-errors".to_string(),
            group: "".to_string(),
            start: "-1h".to_string(),
            end: "".to_string(),
            query: "level=error".to_string(),
        });

        app.open_load_filter_popup();
        // selected index is 0 by default
        app.handle_load_filter_popup_key(KeyCode::Enter);

        assert_eq!(app.filter_start, "-1h");
        assert_eq!(app.filter_end, "");
        assert_eq!(app.filter_query, "level=error");
    }

    #[test]
    fn load_filter_popup_moves_selection_with_up_down() {
        let mut app = app_with_filter_state();
        app.saved_filters.push(SavedFilter {
            name: "first".to_string(),
            group: "".to_string(),
            start: "-5m".to_string(),
            end: "".to_string(),
            query: "a=1".to_string(),
        });
        app.saved_filters.push(SavedFilter {
            name: "second".to_string(),
            group: "".to_string(),
            start: "-15m".to_string(),
            end: "".to_string(),
            query: "b=2".to_string(),
        });

        app.open_load_filter_popup();
        assert_eq!(app.load_filter_selected, 0);

        app.handle_load_filter_popup_key(KeyCode::Down);
        assert_eq!(app.load_filter_selected, 1);

        app.handle_load_filter_popup_key(KeyCode::Up);
        assert_eq!(app.load_filter_selected, 0);
    }

    #[test]
    fn save_and_load_filter_persists_group_selection() {
        // Set up app with two groups and select the second one
        let mut app = app_with_filter_state();
        app.groups = vec![
            "/aws/lambda/first".to_string(),
            "/aws/lambda/second".to_string(),
        ];
        app.selected_group = 1; // "/aws/lambda/second"

        app.filter_start = "-5m".to_string();
        app.filter_end = "".to_string();
        app.filter_query = "routing_id=999".to_string();

        app.open_save_filter_popup();
        app.save_filter_name = "with-group".to_string();
        app.handle_save_filter_popup_key(KeyCode::Enter);

        assert_eq!(app.saved_filters.len(), 1);
        let f = &app.saved_filters[0];
        assert_eq!(f.group, "/aws/lambda/second");

        // Reset selection and then load the filter, it should restore the group
        app.selected_group = 0;
        app.open_load_filter_popup();
        app.handle_load_filter_popup_key(KeyCode::Enter);

        assert_eq!(app.selected_group, 1);
        assert_eq!(app.groups[app.selected_group], "/aws/lambda/second");
    }
}
