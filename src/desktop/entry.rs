use std::path::Path;

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub exec: String,

    pub keywords: Vec<String>,
    pub categories: Vec<String>,
}

impl Entry {
    pub fn from_file(desktop_file_path: &Path) -> Option<Entry> {
        let mut entry = Entry::init();
        let file = ini::ini!(desktop_file_path.to_str()?);

        if !file.contains_key("desktop entry") {
            return None;
        }

        for (key, val_buf) in &file["desktop entry"] {
            let value = val_buf.as_ref()?;

            match key.as_str() {
                "type" => {
                    if value != "Application" {
                        return None;
                    }
                }

                "name" => {
                    entry.name = value.to_owned();
                    continue;
                }

                "exec" => {
                    if value.is_empty() {
                        return None;
                    }

                    if let Some((left, _)) = value.split_once('%') {
                        entry.exec = left.trim_matches('"').to_owned();
                    } else {
                        entry.exec = value.trim_matches('"').to_owned();
                    }
                }

                "keywords" => {
                    entry.keywords = value
                        .split(';')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_owned())
                        .collect();
                }

                "categories" => {
                    entry.categories = value
                        .split(';')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_owned())
                        .collect();
                }

                _ => {
                    continue;
                }
            }
        }

        Some(entry)
    }

    fn init() -> Entry {
        Entry {
            name: String::new(),
            exec: String::new(),
            keywords: Vec::new(),
            categories: Vec::new(),
        }
    }
}
