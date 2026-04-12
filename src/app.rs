use std::{
    io::Error,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    prelude::Backend,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::{desktop, theme};

pub struct Application {
    theme: theme::Theme,

    list_state: ListState,
    query: String,

    entries: desktop::EntryCollection,
    history: desktop::EntryHistory,

    results: Vec<String>,
    exec: Option<String>,
    name: Option<String>,
    is_term: bool,
}

impl Application {
    pub fn new(theme: theme::Theme) -> Application {
        let entries = desktop::EntryCollection::collect();
        let history = desktop::EntryHistory::parse();

        if history.is_err() {
            eprintln!("Failed to parse history file: {}", history.err().unwrap());
            std::process::exit(1);
        }

        let results = entries
            .search("", None)
            .iter()
            .map(|e| e.name.clone())
            .collect();

        Application {
            theme,
            list_state: ListState::default().with_selected(Some(0)),
            query: String::new(),
            entries,
            history: history.ok().unwrap(),
            results,
            exec: None,
            name: None,
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

    fn execute(&mut self) -> bool {
        if let Some(exec) = &self.exec {
            if self.is_term {
                notify_error(Command::new("sh").arg("-c").arg(exec).exec());
                self.save();

                return false;
            }

            let result = Command::new("sh")
                .arg("-c")
                .arg(exec)
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .process_group(0)
                .spawn();

            if let Err(e) = result {
                notify_error(e);
            }
            self.save();

            return true;
        }

        false
    }

    fn save(&mut self) {
        self.history[self.name.as_ref().unwrap()] += 1;
        if let Err(err) = self.history.save() {
            eprintln!("Failed to save history: {}", err);
        }
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
            .search(&self.query, Some(&self.history))
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
                    if key.modifiers == KeyModifiers::CONTROL {
                        match c {
                            'h' => {
                                self.delete_last_word();
                                self.refresh_results();
                                return true;
                            }
                            'q' => {
                                return false;
                            }

                            _ => {}
                        }
                    }

                    self.query.push(c);
                    self.refresh_results();
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
                    if let Some(selected) = self.list_state.selected()
                        && let Some(name) = self.results.get(selected)
                        && let Some(entry) = self.entries.get(name)
                    {
                        self.exec = Some(entry.exec.clone());
                        self.name = Some(name.clone());
                        self.is_term = entry.is_term;
                        return false;
                    }
                }
                KeyCode::Esc => return false,
                _ => {}
            }
        }

        true
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let t = &self.theme;

        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(t.search_bar_height)])
                .split(frame.area());

        let items: Vec<ListItem> = self
            .results
            .iter()
            .map(|name| ListItem::new(Line::from(Span::raw(name.as_str())).style(t.item_normal)))
            .collect();

        let list = List::new(items)
            .highlight_style(t.item_selected)
            .highlight_symbol(&*t.highlight_symbol)
            .block(Block::default().padding(t.list_padding()));

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        let search_text = if self.query.is_empty() {
            Line::from(vec![
                Span::styled(&t.prompt_str, t.prompt),
                Span::styled(&t.placeholder_str, t.placeholder),
            ])
        } else {
            Line::from(vec![
                Span::styled(&t.prompt_str, t.prompt),
                Span::raw(self.query.as_str()),
                Span::styled("⎸", t.cursor),
            ])
        };

        frame.render_widget(
            Paragraph::new(search_text).block(Block::default().padding(t.search_padding())),
            chunks[1],
        );
    }
}

fn notify_error(e: Error) {
    notify_rust::Notification::new()
        .appname("KeDesktop::KeLaunch")
        .summary("Error occured")
        .body(format!("Failed to launch: {e}").as_str())
        .show()
        .unwrap();
}

fn is_word_bound(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
}
