pub struct Locale {
    full: String,
    lang_territory: String,
    lang: String,
}

impl Locale {
    pub fn from_env() -> Self {
        let full = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|var| {
                let val = std::env::var(var).ok()?;
                (!val.is_empty()).then_some(val)
            })
            .unwrap_or_default();

        let lang_territory = full.split('.').next().unwrap_or("").to_owned();
        let lang = lang_territory.split('_').next().unwrap_or("").to_owned();

        Self {
            full,
            lang_territory,
            lang,
        }
    }

    pub fn priority(&self, entry_locale: &str) -> Option<u8> {
        let entry_lang_territory = entry_locale.split('.').next().unwrap_or("");
        let entry_lang = entry_lang_territory.split('_').next().unwrap_or("");

        if entry_locale == self.full {
            Some(2)
        } else if !entry_lang_territory.is_empty() && entry_lang_territory == self.lang_territory {
            Some(1)
        } else if !entry_lang.is_empty() && entry_lang == self.lang {
            Some(0)
        } else {
            None
        }
    }
}
