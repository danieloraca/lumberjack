use std::{io, sync::mpsc, thread, time::Duration};

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

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut groups: Vec<String> = Vec::new();
    groups.push("1".to_string());
    groups.push("2".to_string());

    let mut app = App {
        exit: false,
        lines: Vec::new(),
        groups,
    };

    let app_result = app.run(&mut terminal);

    ratatui::restore();
    app_result
}

pub struct App {
    exit: bool,
    lines: Vec<String>,
    groups: Vec<String>,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
            self.exit = true;
        }

        if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('a') {
            self.lines.push("New line".to_string());
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(area);

        Line::from("Process overview").bold().render(chunks[0], buf);

        let groups_style = Style::default().bg(Color::Rgb(100 as u8, 35 as u8, 200 as u8));
        let groups_block = Block::bordered().title("Groups").style(groups_style);

        let inner = groups_block.inner(chunks[1]);
        groups_block.render(chunks[1], buf);

        for (i, group) in self.groups.iter().take(2).enumerate() {
            Line::from(group.as_str()).render(
                Rect {
                    x: inner.x,
                    y: inner.y + i as u16,
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

    // where
    //     Self: Sized,
    // {
    //     Line::from("Process overview").bold().render(area, buf);
    // }
}
