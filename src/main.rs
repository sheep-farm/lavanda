mod app;
mod audio;
mod config;
mod library;
mod locale;
mod persist;
mod radio;
mod state;
mod ui;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const LICENSE: &str = include_str!("../LICENSE");

fn main() -> iced::Result {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("lavanda {VERSION}");
                std::process::exit(0);
            }
            "--about" => {
                println!("lavanda {VERSION}");
                println!("Native Wayland music player written in Rust.");
                println!("Built for Omarchy / Hyprland. Follows the active theme live.");
                println!();
                println!("Repository : https://github.com/sheep-farm/lavanda");
                println!("License    : MIT");
                std::process::exit(0);
            }
            "--license" => {
                print!("{LICENSE}");
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!("lavanda {VERSION}");
                println!("Native Wayland music player for Omarchy / Hyprland.");
                println!();
                println!("USAGE:");
                println!("    lavanda [OPTIONS]");
                println!();
                println!("OPTIONS:");
                println!("    -h, --help       Print this help message");
                println!("    -V, --version    Print version");
                println!("        --about      About lavanda");
                println!("        --license    Print the MIT license");
                println!();
                println!("CONFIG:  ~/.config/lavanda/config.toml");
                println!("STATE:   ~/.config/lavanda/state.toml");
                std::process::exit(0);
            }
            unknown => {
                eprintln!("lavanda: unknown option '{unknown}'");
                eprintln!("Try 'lavanda --help' for usage.");
                std::process::exit(1);
            }
        }
    }

    config::load();
    locale::load();
    ui::theme::load_system_theme();
    persist::init();
    app::run()
}
