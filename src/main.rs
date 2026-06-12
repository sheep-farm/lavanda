mod app;
mod audio;
mod config;
mod library;
mod locale;
mod ui;

fn main() -> iced::Result {
    config::load(); // primeiro: define music_dir, language, volume…
    locale::load(); // usa config.language
    ui::theme::load_system_theme();
    app::run()
}
