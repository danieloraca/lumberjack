use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatchlogs as cwl;
use aws_types::region::Region;
use crossterm::event;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Margin},
    prelude::{Buffer, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Gauge, Widget},
};
use std::{env, io, sync::mpsc, thread, time::Duration};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Focus {
    Groups,
    Filter,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FilterField {
    Start,
    End,
    Query,
    Search,
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let app_title: String = "Lumberjack".to_string();
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

    let mut app = App {
        app_title,
        exit: false,
        lines: Vec::new(),
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
    };

    let app_result = app.run(&mut terminal);

    ratatui::restore();
    app_result
}

pub struct App {
    app_title: String,
    exit: bool,
    lines: Vec<String>,
    groups: Vec<String>,
    selected_group: usize,
    groups_scroll: usize,
    profile: String,
    region: String,
    focus: Focus,
    filter_start: String,
    filter_end: String,
    filter_query: String,
    filter_field: FilterField,
    editing: bool,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let event::Event::Key(key_event) = event::read()? {
                    self.handle_key_event(key_event)?;
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(&*self, frame.area());
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        if key_event.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key_event.code {
            KeyCode::Char('q') if !self.editing => self.exit = true,

            // Switch focus between left and right panes
            KeyCode::Tab if !self.editing => {
                self.focus = match self.focus {
                    Focus::Groups => Focus::Filter,
                    Focus::Filter => Focus::Groups,
                };
            }

            // ESC cancels editing
            KeyCode::Esc => {
                self.editing = false;
            }

            // While editing: text input goes into the active field
            KeyCode::Backspace if self.editing => {
                self.active_field_mut().pop();
            }
            KeyCode::Char(c) if self.editing => {
                self.active_field_mut().push(c);
            }

            // Enter: start/stop editing, or activate Search
            KeyCode::Enter => {
                if self.focus == Focus::Filter
                    && self.filter_field == FilterField::Search
                    && !self.editing
                {
                    self.start_search(); // stub for now
                } else {
                    self.editing = !self.editing;
                }
            }

            // Navigation when NOT editing
            KeyCode::Up if !self.editing => match self.focus {
                Focus::Groups => self.groups_up(),
                Focus::Filter => self.filter_prev(),
            },

            KeyCode::Down if !self.editing => match self.focus {
                Focus::Groups => self.groups_down(),
                Focus::Filter => self.filter_next(),
            },

            _ => {}
        }

        Ok(())
    }

    fn visible_group_rows(&self) -> usize {
        4
    }

    fn clamp_groups_scroll(&mut self, visible_rows: usize) {
        if self.groups.is_empty() {
            self.groups_scroll = 0;
            self.selected_group = 0;
            return;
        }

        if self.selected_group < self.groups_scroll {
            self.groups_scroll = self.selected_group;
        } else if self.selected_group >= self.groups_scroll + visible_rows {
            self.groups_scroll = self.selected_group + 1 - visible_rows;
        }

        let max_scroll = self.groups.len().saturating_sub(visible_rows);
        self.groups_scroll = self.groups_scroll.min(max_scroll);
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.filter_field {
            FilterField::Start => &mut self.filter_start,
            FilterField::End => &mut self.filter_end,
            FilterField::Query => &mut self.filter_query,
            FilterField::Search => &mut self.filter_query, // unused; won't type into Search
        }
    }

    fn groups_up(&mut self) {
        if !self.groups.is_empty() {
            self.selected_group = self.selected_group.saturating_sub(1);
            self.clamp_groups_scroll(self.visible_group_rows());
        }
    }
    fn groups_down(&mut self) {
        if !self.groups.is_empty() {
            self.selected_group = (self.selected_group + 1).min(self.groups.len() - 1);
            self.clamp_groups_scroll(self.visible_group_rows());
        }
    }

    fn filter_prev(&mut self) {
        self.filter_field = match self.filter_field {
            FilterField::Start => FilterField::Start,
            FilterField::End => FilterField::Start,
            FilterField::Query => FilterField::End,
            FilterField::Search => FilterField::Query,
        };
    }
    fn filter_next(&mut self) {
        self.filter_field = match self.filter_field {
            FilterField::Start => FilterField::End,
            FilterField::End => FilterField::Query,
            FilterField::Query => FilterField::Search,
            FilterField::Search => FilterField::Search,
        };
    }

    fn start_search(&mut self) {
        // for now: just prove wiring works
        let group = self
            .groups
            .get(self.selected_group)
            .cloned()
            .unwrap_or_default();
        self.lines.push(format!(
            "SEARCH group={} start={} end={} query={}",
            group, self.filter_start, self.filter_end, self.filter_query
        ));
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let groups_style = Style::default().bg(Color::Rgb(10 as u8, 35 as u8, 200 as u8));
        let selected_style = groups_style
            .fg(Color::Yellow)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let header_style = Style::default().bg(Color::Rgb(30 as u8, 30 as u8, 30 as u8));
        let footer_style = Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Gray);

        let groups_border = if self.focus == Focus::Groups {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let filter_border = if self.focus == Focus::Filter {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        buf.set_style(chunks[0], header_style);
        buf.set_style(chunks[3], footer_style);

        let header =
            Layout::horizontal([Constraint::Length(20), Constraint::Min(20)]).split(chunks[0]);
        let footer =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]).split(chunks[3]);
        let groups_row =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[1]);

        let header_right_text: String = format!(
            "Profile: {} | Region: {}",
            self.profile.as_str(),
            self.region.as_str(),
        );
        Line::from(self.app_title.as_str())
            .bold()
            .render(header[0], buf);
        Line::from(header_right_text)
            .right_aligned()
            .style(header_style)
            .render(header[1], buf);

        Line::from("Tab Switch pane  ↑↓ Move  Enter Edit/Run  Esc Cancel  q Quit")
            .style(footer_style)
            .render(footer[0], buf);

        Line::from("v0.1.0")
            .right_aligned()
            .style(footer_style)
            .render(footer[1], buf);

        let groups_block = Block::bordered()
            .title("Groups (Tab to switch)")
            .style(groups_style)
            .border_style(groups_border);

        let inner = groups_block.inner(groups_row[0]);
        groups_block.render(groups_row[0], buf);

        let filter_style = Style::default().bg(Color::Rgb(20, 20, 20));
        let filter_block = Block::bordered()
            .title("Filter")
            .style(filter_style)
            .border_style(filter_border);
        let filter_inner = filter_block.inner(groups_row[1]);
        filter_block.render(groups_row[1], buf);

        // Line::from("Filter:")
        //     .style(filter_style.fg(Color::White))
        //     .render(filter_inner, buf);

        let visible_rows = inner.height as usize;
        let start = self.groups_scroll;
        let end = (start + visible_rows).min(self.groups.len());

        for (row, idx) in (start..end).enumerate() {
            let group = &self.groups[idx];

            let selected = idx == self.selected_group;
            let marker = if selected { "(●) " } else { "( ) " };

            let y = inner.y + row as u16;
            Line::from(format!("{marker}{group}"))
                .style(if selected {
                    selected_style
                } else {
                    groups_style
                })
                .render(
                    Rect {
                        x: inner.x,
                        y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );
        }

        for (i, line) in self.lines.iter().enumerate() {
            let y = chunks[2].y + i as u16;
            if y >= chunks[2].bottom() {
                break;
            }

            Line::from(line.as_str()).render(
                Rect {
                    x: chunks[2].x,
                    y,
                    width: chunks[2].width,
                    height: 1,
                },
                buf,
            );
        }

        let mut row_y = filter_inner.y;

        let field_style =
            |field: FilterField, focus: Focus, current: FilterField, editing: bool| {
                if focus == Focus::Filter && field == current {
                    if editing {
                        // actively editing → strong highlight
                        Style::default().bg(Color::Gray).fg(Color::Black)
                    } else {
                        // focused but not editing → white
                        Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20))
                    }
                } else {
                    // unfocused field
                    Style::default()
                        .fg(Color::Rgb(100, 100, 100))
                        .bg(Color::Rgb(20, 20, 20))
                }
            };

        let line = |label: &str, value: &str| format!("{label}: {value}");

        Line::from(line("Start", &self.filter_start))
            .style(field_style(
                FilterField::Start,
                self.focus,
                self.filter_field,
                self.editing,
            ))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        Line::from(line("End", &self.filter_end))
            .style(field_style(
                FilterField::End,
                self.focus,
                self.filter_field,
                self.editing,
            ))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        Line::from(line("Query", &self.filter_query))
            .style(field_style(
                FilterField::Query,
                self.focus,
                self.filter_field,
                self.editing,
            ))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
        row_y += 1;

        // "button"
        let btn = "[ Search ]";
        Line::from(btn)
            .style(field_style(
                FilterField::Search,
                self.focus,
                self.filter_field,
                false,
            ))
            .render(
                Rect {
                    x: filter_inner.x,
                    y: row_y,
                    width: filter_inner.width,
                    height: 1,
                },
                buf,
            );
    }
}

async fn fetch_log_groups(region: &str, profile: &str) -> Result<Vec<String>, cwl::Error> {
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region.to_string())))
        .or_default_provider()
        .or_else(Region::new("eu-west-1"));

    let cfg = aws_config::from_env()
        .region(region_provider)
        .profile_name(profile)
        .load()
        .await;

    let client = cwl::Client::new(&cfg);

    let mut out: Vec<String> = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.describe_log_groups();
        if let Some(token) = &next_token {
            req = req.next_token(token);
        }

        let resp = req.send().await?;

        for g in resp.log_groups() {
            if let Some(name) = g.log_group_name() {
                out.push(name.to_string());
            }
        }

        next_token = resp.next_token().map(|s| s.to_string());
        if next_token.is_none() {
            break;
        }
    }

    out.sort();
    Ok(out)
}
