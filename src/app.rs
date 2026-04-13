use std::{
    io::Error,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
};
use ratatui::prelude::Backend;

use crate::{desktop, theme, ui::AppUI};

pub struct Application {
    theme: theme::Theme,
    ui: AppUI,
    query: String,

    entries: desktop::EntryCollection,
    history: desktop::EntryHistory,

    result: Option<usize>,
}

impl Application {
    pub fn new(theme: theme::Theme) -> Application {
        let history = desktop::EntryHistory::parse().unwrap_or_else(|e| {
            eprintln!("Failed to parse history: {e}");
            std::process::exit(1);
        });

        let entries = desktop::EntryCollection::collect();

        Application {
            theme,
            ui: AppUI::new(),
            query: String::new(),
            entries,
            history,
            result: None,
        }
    }

    pub fn run(&mut self) {
        execute!(std::io::stdout(), EnableMouseCapture).ok();

        ratatui::run(|terminal| {
            terminal.show_cursor().ok();
            loop {
                if !self.on_frame(terminal) {
                    break;
                }
            }
            terminal.hide_cursor().ok();
        });

        execute!(std::io::stdout(), DisableMouseCapture).ok();

        if self.execute() {
            std::process::exit(0);
        }
    }

    fn execute(&mut self) -> bool {
        if let Some(idx) = self.result {
            let entry = &self.entries[idx];

            if entry.is_terminal() {
                notify_error(Command::new("sh").arg("-c").arg(entry.get_exec()).exec());
                self.save();
                return false;
            }

            let result = Command::new("sh")
                .arg("-c")
                .arg(entry.get_exec())
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
        if let Some(idx) = self.result {
            self.history[self.entries[idx].get_name()] += 1;
            if let Err(err) = self.history.save() {
                eprintln!("Failed to save history: {}", err);
            }
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
        self.result = self.entries.search(&self.query, &self.history);
    }

    fn on_frame(&mut self, terminal: &mut ratatui::Terminal<impl Backend>) -> bool {
        let _ = terminal
            .draw(|f| {
                self.ui
                    .draw(f, &self.theme, &self.entries, &self.result, &self.query)
            })
            .ok();

        if !event::poll(std::time::Duration::from_millis(16)).unwrap_or(false) {
            return true;
        }

        let Ok(ev) = event::read() else {
            return true;
        };

        match ev {
            Event::Key(key) => self.handle_key(key),
            _ => true,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers == KeyModifiers::CONTROL {
                    match c {
                        'h' => {
                            self.delete_last_word();
                            self.refresh_results();
                        }
                        'u' => {
                            self.query.clear();
                            self.refresh_results();
                        }
                        'q' => {
                            self.result = None;
                            return false;
                        }
                        _ => {}
                    }
                    return true;
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
            KeyCode::Tab => {
                if let Some(res) = self.result {
                    self.query = self.entries[res].get_localized_name().to_string();
                    self.refresh_results();
                }
            }
            KeyCode::Enter => return false,
            KeyCode::Esc => {
                self.result = None;
                return false;
            }
            _ => {}
        }
        true
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
