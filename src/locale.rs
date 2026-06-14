use std::sync::OnceLock;

static LOCALE: OnceLock<&'static Strings> = OnceLock::new();

pub struct Strings {
    pub no_track: &'static str,
    pub unknown: &'static str,
}

// ── Traduções ─────────────────────────────────────────────────────────────────

static EN: Strings = Strings {
    no_track: "No track",
    unknown: "Unknown",
};

static PT_BR: Strings = Strings {
    no_track: "Nenhuma faixa",
    unknown: "Desconhecido",
};

static ES: Strings = Strings {
    no_track: "Sin pista",
    unknown: "Desconocido",
};

// ── Inicialização ─────────────────────────────────────────────────────────────

pub fn load() {
    LOCALE.get_or_init(detect);
}

pub fn get() -> &'static Strings {
    LOCALE.get_or_init(detect)
}

fn detect() -> &'static Strings {
    let override_lang = crate::config::get().language.clone();
    let lang = if override_lang == "auto" || override_lang.is_empty() {
        std::env::var("LANG")
            .or_else(|_| std::env::var("LANGUAGE"))
            .or_else(|_| std::env::var("LC_ALL"))
            .unwrap_or_default()
    } else {
        override_lang
    };
    let lang = lang.split('.').next().unwrap_or("").to_lowercase();
    if lang.starts_with("pt") {
        &PT_BR
    } else if lang.starts_with("es") {
        &ES
    } else {
        &EN
    }
}
