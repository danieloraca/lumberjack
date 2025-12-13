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
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Char('a') => self.lines.push("New line".to_string()),
            KeyCode::Up => {
                if !self.groups.is_empty() {
                    self.selected_group = self.selected_group.saturating_sub(1);
                    let visible = self.visible_group_rows();
                    self.clamp_groups_scroll(visible);
                }
            }
            KeyCode::Down => {
                if !self.groups.is_empty() {
                    self.selected_group = (self.selected_group + 1).min(self.groups.len() - 1);
                    let visible = self.visible_group_rows();
                    self.clamp_groups_scroll(visible);
                }
            }
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

        buf.set_style(chunks[0], header_style);
        buf.set_style(chunks[3], footer_style);

        let header =
            Layout::horizontal([Constraint::Length(20), Constraint::Min(20)]).split(chunks[0]);
        let footer =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]).split(chunks[3]);

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
            .style(Style::default().fg(Color::Green))
            .render(header[1], buf);

        Line::from("↑↓ Select  q Quit")
            .style(footer_style)
            .render(footer[0], buf);

        Line::from("v0.1.0")
            .right_aligned()
            .style(footer_style)
            .render(footer[1], buf);

        let groups_block = Block::bordered().title("Groups").style(groups_style);
        let inner = groups_block.inner(chunks[1]);
        groups_block.render(chunks[1], buf);

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
