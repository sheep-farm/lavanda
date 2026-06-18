use iced::{font::Weight, Font};

/// Fonte Nerd Font carregada do sistema.
/// Qualquer Nerd Font instalada (JetBrainsMono, FiraCode, Hack, etc.) funciona.
pub const NERD_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
    weight: Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const NERD_FONT_MONO: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font Mono"),
    ..NERD_FONT
};

/// Fonte base da UI — mesma família do Waybar/Omarchy.
pub const UI_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font Mono"),
    weight: Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const UI_FONT_BOLD: Font = Font {
    weight: Weight::Bold,
    ..UI_FONT
};

// Codepoints Font Awesome (Nerd Fonts tier 1 — universais em qualquer Nerd Font)
pub const ICON_PLAY: &str = "\u{f04b}"; //
pub const ICON_PAUSE: &str = "\u{f04c}"; //
pub const ICON_PREV: &str = "\u{f048}"; //
pub const ICON_NEXT: &str = "\u{f051}"; //
pub const ICON_SHUFFLE: &str = "\u{f074}"; //
pub const ICON_REPEAT: &str = "\u{f021}"; //
pub const ICON_VOL_UP: &str = "\u{f028}"; //
pub const ICON_MUSIC: &str = "\u{f001}"; //
pub const ICON_SEARCH: &str = "\u{f002}"; //
pub const ICON_HEART: &str = "\u{f004}"; //
pub const ICON_PLUS: &str = "\u{f067}"; //
pub const ICON_TRASH: &str = "\u{f1f8}"; //
pub const ICON_EDIT: &str = "\u{f044}"; //
pub const ICON_PODIUM: &str = "\u{f0d25}"; //
pub const ICON_CLOCK: &str = "\u{f017}"; //
pub const ICON_LIST: &str = "\u{f0ca}"; //
pub const ICON_CLOSE: &str = "\u{f00d}"; //
pub const ICON_BROADCAST: &str = "\u{f043d}"; // 󰐽 radio tower
pub const ICON_STAR: &str = "\u{f005}"; // ★ favorito de rádio
pub const ICON_CLOUD: &str = "\u{f0c2}"; //  servidor remoto (Jellyfin)
