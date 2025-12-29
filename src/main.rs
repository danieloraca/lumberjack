use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use std::{env, io};

mod app;
mod aws;
mod ui;

use app::{App, FilterField, Focus};
use aws::fetch_log_groups;

const APP_TITLE: &str = "Lumberjack";

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let region = env::args()
        .find_map(|arg| arg.strip_prefix("--region=").map(String::from))
        .unwrap_or_else(|| "eu-west-1".to_string());

    let profile = env::args()
        .find_map(|arg| arg.strip_prefix("--profile=").map(String::from))
        .unwrap_or_else(|| "No Profile Provided".to_string());

    let rt = tokio::runtime::Runtime::new().unwrap();

    let groups = match rt.block_on(fetch_log_groups(&region, &profile)) {
        Ok(g) if !g.is_empty() => g,
        Ok(_) => vec!["(no log groups found)".to_string()],
        Err(e) => vec![format!("(error fetching log groups: {e})")],
    };

    let (search_tx, search_rx) = std::sync::mpsc::channel::<String>();

    let mut app = App {
        app_title: APP_TITLE.to_string(),
        exit: false,
        lines: Vec::new(),
        filter_cursor_pos: 0,
        all_groups: groups.clone(),
        groups,
        selected_group: 0,
        groups_scroll: 0,
        profile,
        region,
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

        search_tx,
        search_rx,
        searching: false,
        dots: 0,
        last_dots: Instant::now(),
        results_scroll: 0,

        tail_mode: false,
        tail_stop: Arc::new(AtomicBool::new(false)),
        status_message: None,
        status_set_at: None,

        json_popup_open: false,
        json_popup_content: String::new(),
        saved_filters: Vec::new(),
        save_filter_popup_open: false,
        save_filter_name: String::new(),
        load_filter_popup_open: false,
        load_filter_selected: 0,
    };

    let app_result = app.run(&mut terminal);

    ratatui::restore();
    app_result
}
