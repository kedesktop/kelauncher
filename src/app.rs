use std::{
    io::Error,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
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

    results: Vec<(usize, usize)>,
    selected: Option<usize>,
}

impl Application {
    pub fn new(theme: theme::Theme) -> Application {
        let history = desktop::EntryHistory::parse().unwrap_or_else(|e| {
            eprintln!("Failed to parse history: {e}");
            std::process::exit(1);
        });

        let entries = desktop::EntryCollection::collect();

        let mut app = Application {
            theme,
            ui: AppUI::new(),
            query: String::new(),
            entries,
            history,
            results: Vec::new(),
            selected: None,
        };

        app.entries.search("", &app.history, &mut app.results);
        app
    }

    pub fn run(&mut self) {
        execute!(std::io::stdout(), EnableMouseCapture).ok();

        ratatui::run(|terminal| {
            loop {
                if !self.on_frame(terminal) {
                    break;
                }
            }
        });

        execute!(std::io::stdout(), DisableMouseCapture).ok();

        if self.execute() {
            std::process::exit(0);
        }
    }

    fn execute(&mut self) -> bool {
        if let Some(idx) = self.selected {
            let entry = &self.entries[idx];

            if entry.is_term {
                notify_error(Command::new("sh").arg("-c").arg(entry.exec.as_ref()).exec());
                self.save();
                return false;
            }

            let result = Command::new("sh")
                .arg("-c")
                .arg(entry.exec.as_ref())
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
        if let Some(idx) = self.selected {
            self.history[&*self.entries[idx].name] += 1;
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
        self.entries
            .search(&self.query, &self.history, &mut self.results);
        self.ui.mark_dirty();
        self.ui.list_state.select(Some(0));
    }

    fn on_frame(&mut self, terminal: &mut ratatui::Terminal<impl Backend>) -> bool {
        self.ui.tick(self.theme.cursor_blink_time);

        let _ = terminal.draw(|f| {
            self.ui
                .draw(f, &self.theme, &self.entries, &self.results, &self.query)
        });

        if event::poll(std::time::Duration::from_millis(16)).unwrap_or(false) {
            if let Ok(ev) = event::read() {
                match ev {
                    Event::Key(key) => return self.handle_key(key),
                    Event::Mouse(mouse) => return self.handle_mouse(mouse),
                    _ => {}
                }
            }
        }
        true
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
                        'q' => return false,
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
                if let Some(&top) = self.results.first() {
                    self.query = self.entries[top.0].name.to_string();
                    self.refresh_results();
                }
            }
            KeyCode::Down => self.ui.select_next(self.results.len()),
            KeyCode::Up => self.ui.select_prev(self.results.len()),
            KeyCode::Enter => return self.select_current(),
            KeyCode::Esc => return false,
            _ => {}
        }
        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> bool {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.ui.scroll_down(self.results.len()),
            MouseEventKind::ScrollUp => self.ui.scroll_up(),
            MouseEventKind::Moved => {
                let top_pad = self.theme.padding_list.2;
                let offset = self.ui.list_state.offset();
                let hovered = mouse.row.saturating_sub(top_pad) as usize + offset;
                if hovered < self.results.len() {
                    self.ui.list_state.select(Some(hovered));
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let top_pad = self.theme.padding_list.2;
                let offset = self.ui.list_state.offset();
                let clicked = mouse.row.saturating_sub(top_pad) as usize + offset;
                if clicked < self.results.len() {
                    self.ui.list_state.select(Some(clicked));
                    return self.select_current();
                }
            }
            _ => {}
        }
        true
    }

    fn select_current(&mut self) -> bool {
        if let Some(selected) = self.ui.list_state.selected() {
            self.selected = self.results.get(selected).map(|r| r.0);
            return false;
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
