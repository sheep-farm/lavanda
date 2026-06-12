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
