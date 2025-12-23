use ratatui::style::{Color, Style};

pub fn header() -> Style {
    Style::default().bg(Color::Rgb(10, 10, 10)).fg(Color::White)
}

pub fn footer() -> Style {
    Style::default().bg(Color::Rgb(10, 10, 10)).fg(Color::Gray)
}

pub fn groups_block(focus: bool) -> Style {
    if focus {
        Style::default().bg(Color::Black).fg(Color::White)
    } else {
        Style::default()
            .bg(Color::Rgb(14, 14, 14))
            .fg(Color::Rgb(140, 140, 140))
    }
}

pub fn group_item(focused: bool) -> Style {
    if focused {
        Style::default().bg(Color::Black).fg(Color::White)
    } else {
        Style::default()
            .bg(Color::Rgb(14, 14, 14))
            .fg(Color::Rgb(140, 140, 140))
    }
}

pub fn groups_selected(focus: bool) -> Style {
    if focus {
        Style::default()
            .bg(Color::Rgb(40, 40, 40))
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().bg(Color::Rgb(18, 18, 18)).fg(Color::White)
    }
}

pub fn filter_block(focus: bool) -> Style {
    if focus {
        Style::default().bg(Color::Rgb(20, 20, 20)).fg(Color::White)
    } else {
        Style::default()
            .bg(Color::Rgb(20, 20, 20))
            .fg(Color::Rgb(140, 140, 140))
    }
}

pub fn results_block(focus: bool) -> Style {
    if focus {
        Style::default().bg(Color::Rgb(5, 5, 5)).fg(Color::White)
    } else {
        Style::default()
            .bg(Color::Rgb(14, 14, 14))
            .fg(Color::Rgb(140, 140, 140))
    }
}

pub fn pane_border(focus: bool) -> Style {
    if focus {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

pub fn default_gray() -> Style {
    Style::default().fg(Color::Gray)
}
