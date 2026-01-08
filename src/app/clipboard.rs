use crate::app::App;
use arboard::Clipboard;
use sqlformat::{self, Dialect, FormatOptions, Indent, QueryParams};
use std::time::Instant;

impl App {
    pub fn results_text(&self) -> String {
        self.state.lines.join("\n")
    }

    pub fn copy_results_to_clipboard(&mut self) {
        let text = self.results_text();
        if text.trim().is_empty() {
            return;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_text(text.clone()).is_ok() {
                self.state.status_message = Some(format!(
                    "Copied {} lines to clipboard",
                    self.state.lines.len()
                ));
                self.state.status_set_at = Some(Instant::now());
            }
        }
    }

    pub fn copy_selected_result_to_clipboard(&mut self) {
        // If the results detail popup is open, try to detect and pretty-print
        // any JSON sql field and copy the formatted SQL only.
        if self.state.results_detail_popup_open {
            let line = self.flatten_selected_line();
            if let Some(formatted) = format_sql_for_clipboard(&line) {
                if let Ok(mut clipboard) = Clipboard::new() {
                    if clipboard.set_text(formatted).is_ok() {
                        self.state.status_message =
                            Some("Copied formatted SQL from popup".to_string());
                        self.state.status_set_at = Some(Instant::now());
                    }
                }
                return;
            }
        }

        // Otherwise, fall back to copying the raw selected line.
        let line = self.flatten_selected_line();
        if line.is_empty() {
            return;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_text(line).is_ok() {
                self.state.status_message = Some("Copied selected result line".to_string());
                self.state.status_set_at = Some(Instant::now());
            }
        }
    }

    fn flatten_selected_line(&self) -> String {
        let mut flat: Vec<String> = Vec::new();
        for entry in &self.state.lines {
            for l in entry.lines() {
                flat.push(l.to_string());
            }
        }

        let idx = match self.state.results_detail_selected_line {
            Some(i) => i,
            None => self.state.results_selected,
        };

        flat.get(idx).cloned().unwrap_or_default()
    }
}

fn format_sql_for_clipboard(line: &str) -> Option<String> {
    // Look for a JSON `"sql": "<value>"` field and pretty-print that value.
    let sql_key_pos = line.find("\"sql\"")?;
    let after_key = &line[sql_key_pos..];
    let colon_rel = after_key.find(':')?;
    let colon_pos = sql_key_pos + colon_rel;

    // Find starting quote of the SQL string
    let chars: Vec<char> = line.chars().collect();
    let mut start = None;
    for i in colon_pos + 1..chars.len() {
        if chars[i].is_whitespace() {
            continue;
        }
        if chars[i] == '"' {
            start = Some(i + 1);
        }
        break;
    }
    let start = start?;

    // Find the closing quote (next unescaped `"`)
    let mut end = None;
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '"' {
            let mut backslashes = 0;
            let mut j = i;
            while j > 0 && chars[j - 1] == '\\' {
                backslashes += 1;
                j -= 1;
            }
            if backslashes % 2 == 0 {
                end = Some(i);
                break;
            }
        }
        i += 1;
    }
    let end = end?;

    let sql_raw: String = chars[start..end].iter().collect();

    // Use sqlformat to pretty-print the SQL value.
    let mut opts = FormatOptions::default();
    opts.indent = Indent::Spaces(2);
    opts.uppercase = Some(true);
    opts.lines_between_queries = 1;
    opts.ignore_case_convert = None;
    opts.inline = false;
    opts.max_inline_block = 50;
    opts.max_inline_arguments = None;
    opts.max_inline_top_level = None;
    opts.joins_as_top_level = false;
    opts.dialect = Dialect::Generic;

    let formatted_sql =
        sqlformat::format(&sql_raw, &QueryParams::None, &opts);

    Some(formatted_sql)
}

#[cfg(test)]
mod tests {
    use crate::app::state::AppState;
    use crate::app::{App, FilterField, Focus};
    use crate::ui::styles::Theme;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, mpsc};
    use std::time::Instant as StdInstant;

    fn app_with_results(lines: Vec<&str>) -> App {
        let (tx, rx) = mpsc::channel();

        let state = AppState {
            app_title: "Test".to_string(),
            theme: Theme::default_dark(),
            theme_name: "dark".to_string(),
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

            searching: false,
            dots: 0,
            last_dots: StdInstant::now(),
            results_scroll: 0,
            results_selected: 0,
            tail_mode: false,

            status_message: None,
            status_set_at: None,

            saved_filters: Vec::new(),
            save_filter_popup_open: false,
            save_filter_name: String::new(),
            load_filter_popup_open: false,
            load_filter_selected: 0,
            results_detail_popup_open: false,
            results_detail_selected_line: None,
            results_detail_scroll: 0,
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
        assert!(app.state.status_message.is_none());
    }
}
