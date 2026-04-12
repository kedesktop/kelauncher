mod app;
mod desktop;
mod theme;
mod ui;

fn main() {
    app::Application::new(theme::Theme::default()).run();
}
