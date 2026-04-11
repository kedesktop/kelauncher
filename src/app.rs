use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Padding, Paragraph},
};

use crate::desktop;

pub struct Application {
    list_state: ListState,
    query: String,
    entries: desktop::EntryCollection,
}

impl Application {
    pub fn new() -> Application {
        Application {
            list_state: ListState::default().with_selected(Some(0)),
            query: String::new(),
            entries: desktop::EntryCollection::collect(),
        }
    }

    pub fn run(&mut self) {
        let mut exec: Option<String> = None;

        ratatui::run(|terminal| {
            loop {
                if let Ok(Event::Key(key)) = event::read() {
                    match key.code {
                        KeyCode::Char(c) => {
                            self.query.push(c);
                            self.list_state.select(Some(0));
                        }
                        KeyCode::Backspace => {
                            self.query.pop();
                            self.list_state.select(Some(0));
                        }
                        KeyCode::Down => self.list_state.select_next(),
                        KeyCode::Up => self.list_state.select_previous(),
                        KeyCode::Enter => {
                            if let Some(selected) = self.list_state.selected() {
                                if let Some(entry) = self.entries.search(&self.query).get(selected)
                                {
                                    exec = Some(entry.exec.clone());
                                }
                                break;
                            }
                        }
                        KeyCode::Esc => break,
                        _ => {}
                    }
                }

                let _ = terminal.draw(|f| self.draw(f));
            }
        });

        if let Some(exec) = exec {
            let _ = Command::new("sh")
                .arg("-c")
                .arg(exec)
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .process_group(0)
                .spawn()
                .expect("failed to run command");
            std::process::exit(0);
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(frame.area());

        let items: Vec<ListItem> = self
            .entries
            .search(&self.query)
            .iter()
            .map(|n| ListItem::new(n.name.as_str()))
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::from_u32(0xD34F6D))
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().padding(Padding::new(2, 2, 1, 0)));

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        let search = Paragraph::new(self.query.as_str())
            .block(Block::default().padding(Padding::new(2, 2, 0, 2)));

        frame.render_widget(search, chunks[1]);
    }
}
