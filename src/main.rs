mod app;
mod desktop;
mod theme;

fn main() {
    app::Application::new(theme::Theme::default()).run();
}
