use super::{FilterField, Focus, SavedFilter};
use crate::ui::styles::Theme;
use std::time::Instant;

pub struct AppState {
    pub app_title: String,
    pub theme: Theme,
    pub theme_name: String,

    pub lines: Vec<String>,
    pub filter_cursor_pos: usize,

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

    pub searching: bool,
    pub dots: usize,
    pub last_dots: Instant,
    pub results_scroll: usize,

    pub tail_mode: bool,

    pub status_message: Option<String>,
    pub status_set_at: Option<Instant>,

    pub saved_filters: Vec<SavedFilter>,
    pub save_filter_popup_open: bool,
    pub save_filter_name: String,
    pub load_filter_popup_open: bool,
    pub load_filter_selected: usize,
}
