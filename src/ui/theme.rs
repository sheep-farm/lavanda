use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use iced::widget::container;
use iced::{Border, Color};

// ── Paleta ───────────────────────────────────────────────────────────────────

static PALETTE: OnceLock<Mutex<Palette>> = OnceLock::new();

struct Palette {
    base: Color,
    mantle: Color,
    surface0: Color,
    overlay0: Color,
    text: Color,
    subtext: Color,
    accent: Color,
    green: Color,
    red: Color,
}

impl Palette {
    fn default_lavender() -> Self {
        Palette {
            base: hex(0x11, 0x11, 0x1b),
            mantle: hex(0x18, 0x18, 0x25),
            surface0: hex(0x31, 0x32, 0x44),
            overlay0: hex(0x6c, 0x70, 0x86),
            text: hex(0xcd, 0xd6, 0xf4),
            subtext: hex(0xa6, 0xad, 0xc8),
            accent: hex(0xcb, 0xa6, 0xf7),
            green: hex(0xa6, 0xe3, 0xa1),
            red: hex(0xf3, 0x8b, 0xa8),
        }
    }
}

fn palette_mutex() -> &'static Mutex<Palette> {
    PALETTE.get_or_init(|| {
        let p = try_load_omarchy_theme().unwrap_or_else(|| {
            eprintln!("lavanda: tema Omarchy não encontrado, usando lavender padrão");
            Palette::default_lavender()
        });
        Mutex::new(p)
    })
}

/// Inicializa a paleta na primeira execução.
pub fn load_system_theme() {
    let _ = palette_mutex();
}

/// Relê o tema do Omarchy em disco e atualiza a paleta sem reiniciar.
pub fn reload_system_theme() {
    if let Some(new) = try_load_omarchy_theme() {
        *palette_mutex().lock().unwrap() = new;
    }
}

/// Retorna o nome do tema atualmente configurado no Omarchy (para detecção de mudanças).
pub fn read_current_theme_name() -> String {
    let home = match home_dir() {
        Some(h) => h,
        None => return String::new(),
    };
    std::fs::read_to_string(home.join(".config/omarchy/current/theme.name"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn try_load_omarchy_theme() -> Option<Palette> {
    let home = home_dir()?;

    let theme_name = std::fs::read_to_string(home.join(".config/omarchy/current/theme.name"))
        .ok()?
        .trim()
        .to_string();

    let user_path = home.join(format!(".config/omarchy/themes/{}/colors.toml", theme_name));
    let system_path = home.join(format!(
        ".local/share/omarchy/themes/{}/colors.toml",
        theme_name
    ));

    let content = std::fs::read_to_string(&user_path)
        .or_else(|_| std::fs::read_to_string(&system_path))
        .ok()?;

    eprintln!("lavanda: carregando tema \"{}\"", theme_name);
    parse_colors_toml(&content)
}

fn parse_colors_toml(content: &str) -> Option<Palette> {
    let mut map: HashMap<String, Color> = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let val = val.trim();

        let hex6 = if let Some(pos) = val.find('#') {
            let after = &val[pos + 1..];
            let end = after
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(after.len());
            after[..end.min(6)].to_string()
        } else {
            val.trim_matches('"').chars().take(6).collect()
        };

        if hex6.len() == 6 {
            if let Some(c) = parse_hex_str(&hex6) {
                map.insert(key, c);
            }
        }
    }

    let bg = *map.get("background")?;
    let fg = *map.get("foreground")?;
    let accent = *map.get("accent")?;

    let c8 = map
        .get("color8")
        .copied()
        .unwrap_or_else(|| lerp_color(bg, fg, 0.3));

    let is_dark = luminance(bg) < 0.5;
    let (mantle, surface0) = if is_dark {
        (lerp_color(bg, c8, 0.10), lerp_color(bg, c8, 0.40))
    } else {
        (lerp_color(bg, fg, 0.05), lerp_color(bg, fg, 0.18))
    };

    Some(Palette {
        base: bg,
        mantle,
        surface0,
        overlay0: c8,
        text: fg,
        subtext: map
            .get("color15")
            .copied()
            .unwrap_or_else(|| lerp_color(fg, c8, 0.3)),
        accent,
        red: map
            .get("color1")
            .copied()
            .unwrap_or_else(|| hex(0xf3, 0x8b, 0xa8)),
        green: map
            .get("color2")
            .copied()
            .unwrap_or_else(|| hex(0xa6, 0xe3, 0xa1)),
    })
}

fn parse_hex_str(s: &str) -> Option<Color> {
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(hex(r, g, b))
}

fn luminance(c: Color) -> f32 {
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

// ── Acessores de cor ─────────────────────────────────────────────────────────

macro_rules! color_fn {
    ($name:ident, $field:ident) => {
        pub fn $name() -> Color {
            palette_mutex().lock().unwrap().$field
        }
    };
}

color_fn!(base, base);
color_fn!(mantle, mantle);
color_fn!(surface0, surface0);
color_fn!(overlay0, overlay0);
color_fn!(text, text);
color_fn!(subtext, subtext);
color_fn!(accent, accent);
color_fn!(green, green);
color_fn!(red, red);

// ── Utilitários ──────────────────────────────────────────────────────────────

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn hex(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

// ── Estilos de Container ──────────────────────────────────────────────────────

pub fn card(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(mantle())),
        border: Border {
            color: surface0(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn header(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(mantle())),
        ..Default::default()
    }
}

pub fn sidebar(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(mantle())),
        border: Border {
            color: surface0(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn selected_row(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(with_alpha(accent(), 0.15))),
        border: Border {
            color: with_alpha(accent(), 0.4),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn cursor_row(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(with_alpha(surface0(), 0.7))),
        border: Border {
            color: with_alpha(accent(), 0.6),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn player_panel(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(mantle())),
        border: Border {
            color: surface0(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn album_header(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(with_alpha(surface0(), 0.5))),
        border: Border {
            color: with_alpha(accent(), 0.2),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
