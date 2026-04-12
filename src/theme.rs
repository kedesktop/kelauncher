use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub prompt: Style,
    pub cursor: Style,
    pub count: Style,
    pub placeholder: Style,
    pub item_normal: Style,
    pub item_selected: Style,
    pub highlight_symbol: String,
    pub padding_list: (u16, u16, u16, u16),
    pub padding_search: (u16, u16, u16, u16),
    pub prompt_str: String,
    pub placeholder_str: String,
    pub search_bar_height: u16,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            prompt: Style::default()
                .fg(Color::from_u32(0x768fa2))
                .add_modifier(Modifier::BOLD),
            cursor: Style::default()
                .fg(Color::from_u32(0x768fa2))
                .add_modifier(Modifier::BOLD),
            count: Style::default().fg(Color::from_u32(0x555555)),
            placeholder: Style::default().fg(Color::from_u32(0x555555)),
            item_normal: Style::default().fg(Color::from_u32(0x777777)),
            item_selected: Style::default().fg(Color::White),
            highlight_symbol: String::new(),
            padding_list: (3, 3, 1, 0),
            padding_search: (2, 2, 1, 0),
            prompt_str: "> ".to_owned(),
            placeholder_str: "search...".to_owned(),
            search_bar_height: 2,
        }
    }
}

impl Theme {
    pub fn list_padding(&self) -> ratatui::widgets::Padding {
        let (l, r, t, b) = self.padding_list;
        ratatui::widgets::Padding::new(l, r, t, b)
    }

    pub fn search_padding(&self) -> ratatui::widgets::Padding {
        let (l, r, t, b) = self.padding_search;
        ratatui::widgets::Padding::new(l, r, t, b)
    }
}
