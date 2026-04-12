use std::{
    io::Error,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crossterm::{event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind
}, execute};
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
        let history = desktop::EntryHistory::parse().unwrap_or_else(|e| {
            eprintln!("Failed to parse history: {e}");
            std::process::exit(1);
        });

        let entries = desktop::EntryCollection::collect();
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
            history: history,
            results,
            exec: None,
            name: None,
            is_term: false,
        }
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

        if let Ok(event) = event::read() {
            match event {
                Event::Key(key) => return self.handle_key(key),
                Event::Mouse(mouse) => return self.handle_mouse(mouse),
                _ => {}
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
                            return true;
                        }
                        'q' => {
                            return false;
                        }
                        'u' => {
                            self.query.clear();
                            self.refresh_results();
                            return true;
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
            KeyCode::Tab => {
                if let Some(top) = self.results.first() {
                    self.query = top.clone();
                    self.refresh_results();
                }
            }
            KeyCode::Down => self.select_next(),
            KeyCode::Up => self.select_prev(),
            KeyCode::Enter => return self.select_current(),
            KeyCode::Esc => return false,
            _ => {}
        }

        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> bool {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.select_next(),
            MouseEventKind::ScrollUp => self.select_prev(),
            MouseEventKind::Down(MouseButton::Left) => {
                /* figure out the clicked row */
                let t = &self.theme;
                let (_, top_pad, _) = (t.padding_list.0, t.padding_list.2, t.padding_list.3);
                let clicked_row = mouse.row.saturating_sub(top_pad) as usize;

                if clicked_row < self.results.len() {
                    if self.list_state.selected() == Some(clicked_row) {
                        return self.select_current();
                    }
                    self.list_state.select(Some(clicked_row));
                }
            }
            _ => {}
        }
        true
    }

    fn select_next(&mut self) {
        let len = self.results.len();
        if len == 0 {
            return;
        }
        let next = self
            .list_state
            .selected()
            .map(|i| (i + 1) % len)
            .unwrap_or(0);
        self.list_state.select(Some(next));
    }

    fn select_prev(&mut self) {
        let len = self.results.len();
        if len == 0 {
            return;
        }
        let prev = self
            .list_state
            .selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.list_state.select(Some(prev));
    }

    fn select_current(&mut self) -> bool {
        if let Some(selected) = self.list_state.selected()
            && let Some(name) = self.results.get(selected).cloned()
            && let Some(entry) = self.entries.get(&name)
        {
            self.exec = Some(entry.exec.clone());
            self.name = Some(name);
            self.is_term = entry.is_term;
            return false;
        }
        true
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let t = &self.theme;

        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(t.search_bar_height)])
                .split(frame.area());

        let count_str = format!("{}/{}", self.results.len(), self.entries.len());
        let count_width = count_str.len() as u16 + t.padding_search.0 + t.padding_search.1;

        let search_chunks =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(count_width)])
                .split(chunks[1]);

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
            search_chunks[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(count_str, t.count)))
                .alignment(ratatui::layout::Alignment::Right)
                .block(Block::default().padding(t.search_padding())),
            search_chunks[1],
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
