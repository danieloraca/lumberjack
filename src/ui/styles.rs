use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Debug)]
pub struct Theme {
    pub header: Style,
    pub footer: Style,

    pub groups_block_focused: Style,
    pub groups_block_unfocused: Style,
    pub groups_item_focused: Style,
    pub groups_item_unfocused: Style,
    pub groups_selected_focused: Style,
    pub groups_selected_unfocused: Style,

    pub filter_block_focused: Style,
    pub filter_block_unfocused: Style,

    pub results_block_focused: Style,
    pub results_block_unfocused: Style,

    pub pane_border_focused: Style,
    pub pane_border_unfocused: Style,

    pub default_gray: Style,
    pub filter_field_active_editing: Style,
    pub filter_field_active_idle: Style,
    pub filter_field_inactive: Style,

    pub popup_block: Style,
    pub popup_border: Style,
    pub presets_hint: Style,
    pub cursor: Style,
}

impl Theme {
    pub fn default_dark() -> Self {
        Theme {
            header: Style::default().bg(Color::Rgb(10, 10, 10)).fg(Color::White),
            footer: Style::default().bg(Color::Rgb(10, 10, 10)).fg(Color::Gray),
            groups_block_focused: Style::default().bg(Color::Black).fg(Color::White),
            groups_block_unfocused: Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140)),

            groups_item_focused: Style::default().bg(Color::Black).fg(Color::White),
            groups_item_unfocused: Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140)),

            groups_selected_focused: Style::default()
                .bg(Color::Rgb(40, 40, 40))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            groups_selected_unfocused: Style::default().bg(Color::Rgb(18, 18, 18)).fg(Color::White),

            filter_block_focused: Style::default().bg(Color::Rgb(20, 20, 20)).fg(Color::White),
            filter_block_unfocused: Style::default()
                .bg(Color::Rgb(20, 20, 20))
                .fg(Color::Rgb(140, 140, 140)),

            results_block_focused: Style::default().bg(Color::Rgb(5, 5, 5)).fg(Color::White),
            results_block_unfocused: Style::default()
                .bg(Color::Rgb(14, 14, 14))
                .fg(Color::Rgb(140, 140, 140)),

            pane_border_focused: Style::default().fg(Color::Yellow),
            pane_border_unfocused: Style::default(),

            default_gray: Style::default().fg(Color::Gray),

            filter_field_active_editing: Style::default().bg(Color::Gray).fg(Color::Black),
            filter_field_active_idle: Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20)),
            filter_field_inactive: Style::default()
                .fg(Color::Rgb(100, 100, 100))
                .bg(Color::Rgb(20, 20, 20)),

            popup_block: Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White),
            popup_border: Style::default().fg(Color::Yellow),

            presets_hint: Style::default().fg(Color::Rgb(50, 50, 50)),
            cursor: Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20)),
        }
    }

    pub fn light() -> Self {
        // Start from dark to fill all fields, then override what we care about.
        let mut t = Theme::default_dark();

        let bg = Color::Rgb(240, 240, 240);
        let bg_alt = Color::Rgb(230, 230, 230);
        let text = Color::Rgb(30, 30, 30);

        // Header / footer
        t.header = Style::default().bg(bg).fg(text);
        t.footer = Style::default().bg(bg).fg(text);

        // Groups block background
        t.groups_block_focused = Style::default().bg(bg_alt).fg(text);
        t.groups_block_unfocused = Style::default().bg(bg_alt).fg(text);

        // Group items
        t.groups_item_unfocused = Style::default().bg(bg_alt).fg(text);
        t.groups_item_focused = t.groups_item_unfocused;

        t.groups_selected_focused = Style::default()
            .bg(Color::Rgb(210, 210, 210))
            .fg(text)
            .add_modifier(Modifier::BOLD);
        t.groups_selected_unfocused = Style::default().bg(Color::Rgb(220, 220, 220)).fg(text);

        // Filter block
        t.filter_block_focused = Style::default().bg(bg).fg(text);
        t.filter_block_unfocused = Style::default().bg(bg).fg(text);

        // Results block
        t.results_block_focused = Style::default().bg(bg).fg(text);
        t.results_block_unfocused = Style::default().bg(bg).fg(text);

        // Borders
        t.pane_border_focused = Style::default().fg(Color::Rgb(80, 80, 80));
        t.pane_border_unfocused = Style::default().fg(Color::Rgb(180, 180, 180));

        // Default gray text (used for "Searching..." etc.)
        t.default_gray = Style::default().fg(Color::Rgb(120, 120, 120));

        // Filter fields
        t.filter_field_inactive = Style::default().bg(bg).fg(Color::Rgb(120, 120, 120));
        t.filter_field_active_idle = Style::default().bg(Color::Rgb(220, 220, 220)).fg(text);
        t.filter_field_active_editing = Style::default().bg(Color::Rgb(200, 200, 200)).fg(text);

        // Popup
        t.popup_block = Style::default().bg(Color::Rgb(245, 245, 245)).fg(text);
        t.popup_border = Style::default().fg(Color::Rgb(100, 100, 100));

        // Presets hint, cursor
        t.presets_hint = Style::default().fg(Color::Rgb(100, 100, 100));
        t.cursor = Style::default().fg(text).bg(Color::Rgb(220, 220, 220));

        t
    }

    pub fn green() -> Self {
        let mut t = Theme::default_dark();

        let green = Color::Rgb(160, 255, 0);
        let dark_bg = Color::Black;
        let band_bg = Color::Rgb(0, 40, 0);
        let bright_bg = Color::Rgb(0, 90, 0);

        t.header = Style::default()
            .bg(dark_bg)
            .fg(green)
            .add_modifier(Modifier::BOLD);
        t.footer = Style::default().bg(dark_bg).fg(green);

        t.groups_block_focused = Style::default().bg(dark_bg).fg(green);
        // Unselected items: plain phosphor style
        t.groups_item_unfocused = Style::default().bg(dark_bg).fg(green);
        // Selected item (when Groups pane is focused): brighter band with bold
        t.groups_selected_focused = Style::default()
            .bg(bright_bg)
            .fg(green)
            .add_modifier(Modifier::BOLD);

        // t.groups_item_focused = Style::default().bg(dark_bg).fg(green);
        // t.groups_item_unfocused = Style::default().bg(dark_bg).fg(green);
        t.groups_item_unfocused = Style::default().bg(dark_bg).fg(green);
        t.groups_item_focused = t.groups_item_unfocused;

        t.groups_selected_unfocused = Style::default().bg(dark_bg).fg(green);

        t.filter_block_focused = Style::default().bg(dark_bg).fg(green);
        t.filter_block_unfocused = Style::default().bg(dark_bg).fg(green);

        t.results_block_focused = Style::default().bg(dark_bg).fg(green);
        t.results_block_unfocused = Style::default().bg(dark_bg).fg(green);

        t.pane_border_focused = Style::default().fg(green);
        t.pane_border_unfocused = Style::default().fg(green);

        t.default_gray = Style::default().fg(green);

        // Inactive filter fields: black bg, green text
        t.filter_field_inactive = Style::default().bg(dark_bg).fg(green);

        // Active idle filter field: dark green band
        t.filter_field_active_idle = Style::default().bg(band_bg).fg(green);

        // Active editing filter field: brighter band, maybe bold
        t.filter_field_active_editing = Style::default()
            .bg(bright_bg)
            .fg(green)
            .add_modifier(Modifier::BOLD);

        t.popup_block = Style::default().bg(dark_bg).fg(green);
        t.popup_border = Style::default().fg(green);

        t.presets_hint = Style::default().fg(green);
        t.cursor = Style::default().fg(green).bg(dark_bg);

        t
    }
}

pub fn groups_block(theme: &Theme, focus: bool) -> Style {
    if focus {
        theme.groups_block_focused
    } else {
        theme.groups_block_unfocused
    }
}

pub fn group_item(theme: &Theme, focused: bool) -> Style {
    if focused {
        theme.groups_item_focused
    } else {
        theme.groups_item_unfocused
    }
}

pub fn groups_selected(theme: &Theme, focus: bool) -> Style {
    if focus {
        theme.groups_selected_focused
    } else {
        theme.groups_selected_unfocused
    }
}

pub fn filter_block(theme: &Theme, focus: bool) -> Style {
    if focus {
        theme.filter_block_focused
    } else {
        theme.filter_block_unfocused
    }
}

pub fn results_block(theme: &Theme, focus: bool) -> Style {
    if focus {
        theme.results_block_focused
    } else {
        theme.results_block_unfocused
    }
}

pub fn pane_border(theme: &Theme, focus: bool) -> Style {
    if focus {
        theme.pane_border_focused
    } else {
        theme.pane_border_unfocused
    }
}

pub fn default_gray(theme: &Theme) -> Style {
    theme.default_gray
}

pub fn filter_field(theme: &Theme, field_is_active: bool, editing: bool) -> Style {
    if field_is_active {
        if editing {
            theme.filter_field_active_editing
        } else {
            theme.filter_field_active_idle
        }
    } else {
        theme.filter_field_inactive
    }
}

pub fn popup_block(theme: &Theme) -> Style {
    theme.popup_block
}

pub fn popup_border(theme: &Theme) -> Style {
    theme.popup_border
}

pub fn presets_hint(theme: &Theme) -> Style {
    theme.presets_hint
}

pub fn cursor(theme: &Theme) -> Style {
    theme.cursor
}
