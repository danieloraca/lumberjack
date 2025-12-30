use crate::app::Focus;
use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Debug)]
pub struct Theme {
    pub header: Style,
    pub footer: Style,
}

impl Theme {
    pub fn default_dark() -> Self {
        Theme {
            header: Style::default().bg(Color::Rgb(10, 10, 10)).fg(Color::White),
            footer: Style::default().bg(Color::Rgb(10, 10, 10)).fg(Color::Gray),
        }
    }
}

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

pub fn filter_field(field_is_active: bool, editing: bool) -> Style {
    if field_is_active {
        if editing {
            Style::default().bg(Color::Gray).fg(Color::Black)
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20))
        }
    } else {
        Style::default()
            .fg(Color::Rgb(100, 100, 100))
            .bg(Color::Rgb(20, 20, 20))
    }
}

pub fn popup_block() -> Style {
    Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White)
}

pub fn popup_border() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn presets_hint() -> Style {
    Style::default().fg(Color::Rgb(50, 50, 50))
}

pub fn cursor() -> Style {
    Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20))
}
