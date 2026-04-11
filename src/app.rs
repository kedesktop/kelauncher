use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    prelude::Backend,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Padding, Paragraph},
};

use crate::desktop;

pub struct Application {
    list_state: ListState,
    query: String,
    entries: desktop::EntryCollection,

    results: Vec<String>,
    exec: Option<String>,
    is_term: bool,
}

impl Application {
    pub fn new() -> Application {
        let entries = desktop::EntryCollection::collect();
        let results = entries.search("").iter().map(|e| e.name.clone()).collect();

        Application {
            list_state: ListState::default().with_selected(Some(0)),
            query: String::new(),
            entries,
            results,
            exec: None,
            is_term: false,
        }
    }

    pub fn run(&mut self) {
        ratatui::run(|terminal| {
            loop {
                if !self.on_frame(terminal) {
                    break;
                }
            }
        });

        if self.execute() {
            std::process::exit(0);
        }
    }

    fn execute(&self) -> bool {
        if let Some(exec) = &self.exec {
            if self.is_term {
                let _ = Command::new("sh").arg("-c").arg(exec).exec();
                return false;
            } else {
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(exec)
                    .stderr(Stdio::null())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .process_group(0)
                    .spawn()
                    .expect("failed to run command");

                return true;
            }
        }
        return false;
    }

    fn delete_last_word(&mut self) {
        let chars: Vec<char> = self.query.chars().collect();
        let mut index = chars.len();

        if index > 0 {
            let is_bound = is_word_bound(chars[index - 1]);
            while index > 0 && is_word_bound(chars[index - 1]) == is_bound {
                index -= 1;
            }
        }

        self.query = chars[..index].iter().collect();
    }

    fn refresh_results(&mut self) {
        self.results = self
            .entries
            .search(&self.query)
            .iter()
            .map(|e| e.name.clone())
            .collect();
        self.list_state.select(Some(0));
    }

    fn on_frame(&mut self, terminal: &mut ratatui::Terminal<impl Backend>) -> bool {
        let _ = terminal.draw(|f| self.draw(f));

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char(c) => {
                    if c == 'h' && key.modifiers == KeyModifiers::CONTROL {
                        self.delete_last_word();
                        self.refresh_results();
                    } else {
                        self.query.push(c);
                        self.refresh_results();
                    }
                }
                KeyCode::Backspace => {
                    if key.modifiers == KeyModifiers::CONTROL {
                        self.delete_last_word();
                    } else {
                        self.query.pop();
                    }
                    self.refresh_results();
                }
                KeyCode::Down => {
                    let len = self.results.len();
                    if len == 0 {
                        return true;
                    }
                    let next = self
                        .list_state
                        .selected()
                        .map(|i| (i + 1) % len)
                        .unwrap_or(0);
                    self.list_state.select(Some(next));
                }
                KeyCode::Up => {
                    let len = self.results.len();
                    if len == 0 {
                        return true;
                    }
                    let prev = self
                        .list_state
                        .selected()
                        .map(|i| if i == 0 { len - 1 } else { i - 1 })
                        .unwrap_or(0);
                    self.list_state.select(Some(prev));
                }
                KeyCode::Enter => {
                    if let Some(selected) = self.list_state.selected() {
                        if let Some(entry) = self.entries.search(&self.query).get(selected) {
                            self.exec = Some(entry.exec.clone());
                            self.is_term = entry.is_term;
                            return false;
                        }
                    }
                }
                KeyCode::Esc => return false,
                _ => {}
            }
        }

        true
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(frame.area());

        let items: Vec<ListItem> = self
            .results
            .iter()
            .map(|name| {
                ListItem::new(
                    Line::from(Span::raw(name.as_str()))
                        .style(Style::new().fg(Color::from_u32(0x777777))),
                )
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().fg(Color::White))
            .block(Block::default().padding(Padding::new(3, 3, 1, 0)));

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        let search_text = if self.query.is_empty() {
            Line::from(vec![
                Span::styled("> ", Style::default().bold().fg(Color::Cyan)),
                Span::styled("search...", Style::default().fg(Color::from_u32(0x555555))),
            ])
        } else {
            Line::from(vec![
                Span::styled("> ", Style::default().bold().fg(Color::Cyan)),
                Span::raw(self.query.as_str()),
                Span::styled("⎸", Style::default().bold().fg(Color::Cyan)),
            ])
        };

        frame.render_widget(
            Paragraph::new(search_text).block(Block::default().padding(Padding::new(2, 2, 1, 0))),
            chunks[1],
        );
    }
}

fn is_word_bound(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
}
