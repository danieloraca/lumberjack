use std::time::Instant;

use arboard::Clipboard;

use crate::app::App;

impl App {
    pub fn results_text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn copy_results_to_clipboard(&mut self) {
        let text = self.results_text();
        if text.trim().is_empty() {
            return;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_text(text.clone()).is_ok() {
                self.status_message =
                    Some(format!("Copied {} lines to clipboard", self.lines.len()));
                self.status_set_at = Some(Instant::now());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, FilterField, Focus};
    use std::sync::mpsc;
    use std::time::Instant as StdInstant;

    fn app_with_results(lines: Vec<&str>) -> App {
        let (tx, rx) = mpsc::channel();

        App {
            app_title: "Test".to_string(),
            exit: false,
            lines: lines.into_iter().map(|s| s.to_string()).collect(),
            filter_cursor_pos: 0,

            all_groups: Vec::new(),
            groups: Vec::new(),
            selected_group: 0,
            groups_scroll: 0,

            profile: "test-profile".to_string(),
            region: "eu-west-1".to_string(),
            focus: Focus::Results,

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

    #[test]
    fn results_text_joins_lines_with_newlines() {
        let app = app_with_results(vec!["line1", "line2", "line3"]);
        let text = app.results_text();
        assert_eq!(text, "line1\nline2\nline3");
    }

    #[test]
    fn results_text_handles_embedded_newlines() {
        let app = app_with_results(vec!["line1a\nline1b", "line2"]);
        let text = app.results_text();
        // Outer join adds one newline between entries
        assert_eq!(text, "line1a\nline1b\nline2");
    }

    #[test]
    fn copy_results_to_clipboard_does_nothing_when_empty() {
        let mut app = app_with_results(Vec::new());
        // Should not panic or set a status when there is nothing to copy.
        app.copy_results_to_clipboard();
        assert!(app.status_message.is_none());
    }
}
