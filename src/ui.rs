use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::{desktop, theme};

pub struct AppUI {}

impl AppUI {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        theme: &theme::Theme,
        entries: &desktop::EntryCollection,
        result: &Option<usize>,
        query: &str,
    ) {
        let t = theme;

        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(t.search_bar_height)])
                .split(frame.area());

        if let Some(i) = result {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    entries[*i].get_localized_name().to_string(),
                    t.item,
                )))
                .block(Block::default().padding(t.list_padding())),
                chunks[0],
            );
        }

        let search_text = if query.is_empty() {
            Line::from(vec![
                Span::styled(&t.prompt_str, t.prompt),
                Span::styled(&t.placeholder_str, t.placeholder),
            ])
        } else {
            Line::from(vec![
                Span::styled(&t.prompt_str, t.prompt),
                Span::raw(query.to_owned()),
            ])
        };

        frame.render_widget(
            Paragraph::new(search_text).block(Block::default().padding(t.search_padding())),
            chunks[1],
        );

        let cursor_x = chunks[1].x
            + t.padding_search.0
            + t.prompt_str.chars().count() as u16
            + query.chars().count() as u16;

        let cursor_y = chunks[1].y + t.padding_search.2;

        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
