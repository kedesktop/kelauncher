use super::locale::Locale;
use std::path::Path;

pub struct Entry {
    name: Box<str>,

    #[allow(unused)]
    localized_name: Option<Box<str>>,

    keywords: Box<[Box<str>]>,

    exec: Box<str>,

    terminal: bool,
}

impl Entry {
    pub fn from_file(desktop_file_path: &Path, locale: &Locale) -> Option<Entry> {
        let content = std::fs::read_to_string(desktop_file_path).ok()?;
        parse(&content, locale)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_localized_name(&self) -> &str {
        #[cfg(feature = "localized")]
        return self.localized_name.as_deref().unwrap_or(&self.name);

        #[cfg(not(feature = "localized"))]
        self.get_name()
    }

    pub fn get_keywords(&self) -> &[Box<str>] {
        &self.keywords
    }

    pub fn get_exec(&self) -> &str {
        &self.exec
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
}

fn parse(content: &str, locale: &Locale) -> Option<Entry> {
    let mut name: Option<Box<str>> = None;
    let mut localized_name: Option<Box<str>> = None;
    let mut localized_name_priority = 0u8;
    let mut exec: Option<Box<str>> = None;
    let mut keywords: Option<Box<[Box<str>]>> = None;
    let mut localized_keywords_priority: Option<u8> = None;
    let mut terminal = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry || line.starts_with('#') || line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let (key, value) = (key.trim(), value.trim());

        if key.contains('[') {
            let Some((base, entry_locale)) = parse_localized_key(key) else {
                continue;
            };

            let Some(priority) = locale.priority(entry_locale) else {
                continue;
            };

            match base {
                "Name" => {
                    if localized_name.is_none() || priority > localized_name_priority {
                        localized_name = Some(value.into());
                        localized_name_priority = priority;
                    }
                }
                "Keywords" => {
                    if localized_keywords_priority.is_none_or(|p| priority > p) {
                        keywords = Some(parse_keywords(value));
                        localized_keywords_priority = Some(priority);
                    }
                }
                _ => {}
            }
            continue;
        }

        match key {
            "Type" => {
                if value != "Application" {
                    return None;
                }
            }
            "Name" => name = Some(value.into()),
            "Exec" => {
                if value.is_empty() {
                    return None;
                }
                exec = Some(strip_field_codes(value).trim().trim_matches('"').into());
            }
            "NoDisplay" => {
                if value == "true" {
                    return None;
                }
            }
            "Keywords" => {
                if localized_keywords_priority.is_none() {
                    keywords = Some(parse_keywords(value));
                }
            }
            "Terminal" => terminal = value == "true",
            _ => {}
        }
    }

    let (Some(name), Some(exec)) = (name, exec) else {
        return None;
    };

    if name.is_empty() || !is_exec_valid(&exec) {
        return None;
    }

    Some(Entry {
        name,
        localized_name,
        exec,
        keywords: keywords.unwrap_or_default(),
        terminal,
    })
}

/** Splits `"Name[fr_FR]"` into `("Name", "fr_FR")`. */
fn parse_localized_key(key: &str) -> Option<(&str, &str)> {
    let bracket = key.find('[')?;
    let base = &key[..bracket];
    let locale = key[bracket + 1..].strip_suffix(']')?;
    Some((base, locale))
}

fn parse_keywords(value: &str) -> Box<[Box<str>]> {
    value
        .split(';')
        .map(|s| Box::from(s.trim()))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn is_exec_valid(exec: &str) -> bool {
    if exec.is_empty() {
        return false;
    }

    let mut parts = exec.split_whitespace();

    let Some(cmd) = parts.next() else {
        return false;
    };

    if cmd.starts_with('/') {
        return std::fs::metadata(cmd)
            .map(|m| {
                use std::os::unix::fs::PermissionsExt;
                m.is_file() && m.permissions().mode() & 0o111 != 0
            })
            .unwrap_or(false);
    }

    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|dir| std::path::Path::new(dir).join(cmd))
        .any(|path| {
            std::fs::metadata(&path)
                .map(|m| {
                    use std::os::unix::fs::PermissionsExt;
                    m.is_file() && m.permissions().mode() & 0o111 != 0
                })
                .unwrap_or(false)
        })
}

fn strip_field_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            result.push(c);
            continue;
        }

        match chars.peek() {
            Some(&code) if "fFuUdDnNickvm".contains(code) => {
                chars.next();
            }
            Some(&'%') => {
                chars.next();
                result.push('%');
            }
            _ => result.push('%'),
        }
    }

    result
}
