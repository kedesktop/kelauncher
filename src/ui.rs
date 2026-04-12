use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::{desktop, theme};

pub struct AppUI {
    pub list_state: ListState,
    cached_items: Vec<ListItem<'static>>,
    items_dirty: bool,

    cursor_visible: bool,
    last_blink: std::time::Instant,
}

impl AppUI {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default().with_selected(Some(0)),
            cached_items: Vec::new(),
            items_dirty: true,
            cursor_visible: true,
            last_blink: std::time::Instant::now(),
        }
    }

    pub fn mark_dirty(&mut self) {
        self.items_dirty = true;
    }

    /** Is called once every frame, will return true if a redraw is needed */
    pub fn tick(&mut self, blink_time: u64) -> bool {
        if self.last_blink.elapsed() >= std::time::Duration::from_millis(blink_time) {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = std::time::Instant::now();
            return true;
        }
        false
    }

    pub fn select_next(&mut self, results_len: usize) {
        if results_len == 0 {
            return;
        }
        let next = self
            .list_state
            .selected()
            .map(|i| (i + 1) % results_len)
            .unwrap_or(0);
        self.list_state.select(Some(next));
    }

    pub fn select_prev(&mut self, results_len: usize) {
        if results_len == 0 {
            return;
        }
        let prev = self
            .list_state
            .selected()
            .map(|i| if i == 0 { results_len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.list_state.select(Some(prev));
    }

    pub fn scroll_down(&mut self, results_len: usize) {
        let max_offset = results_len.saturating_sub(1);
        let offset = self.list_state.offset_mut();
        *offset = (*offset + 1).min(max_offset);
    }

    pub fn scroll_up(&mut self) {
        let offset = self.list_state.offset_mut();
        *offset = offset.saturating_sub(1);
    }

    pub fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        theme: &theme::Theme,
        entries: &desktop::EntryCollection,
        results: &[(usize, usize)],
        query: &str,
    ) {
        if self.items_dirty {
            self.cached_items = results
                .iter()
                .map(|&(i, _)| {
                    ListItem::new(
                        Line::from(Span::raw(entries[i].name.to_string())).style(theme.item_normal),
                    )
                })
                .collect();
            self.items_dirty = false;
        }

        let t = theme;

        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(t.search_bar_height)])
                .split(frame.area());

        let count_str = format!("{}/{}", results.len(), entries.len());
        let count_width = count_str.len() as u16 + t.padding_search.0 + t.padding_search.1;

        let search_chunks =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(count_width)])
                .split(chunks[1]);

        let list = List::new(self.cached_items.clone())
            .highlight_style(t.item_selected)
            .highlight_symbol(&*t.highlight_symbol)
            .block(Block::default().padding(t.list_padding()));

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        let cursor = if self.cursor_visible {
            Span::styled("⎸", t.cursor)
        } else {
            Span::raw(" ")
        };

        let search_text = if query.is_empty() {
            Line::from(vec![
                Span::styled(&t.prompt_str, t.prompt),
                Span::styled(&t.placeholder_str, t.placeholder),
            ])
        } else {
            Line::from(vec![
                Span::styled(&t.prompt_str, t.prompt),
                Span::raw(query.to_owned()),
                cursor,
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
