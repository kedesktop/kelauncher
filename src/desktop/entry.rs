use std::path::Path;

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub exec: String,
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
                    if let Some((left, _)) = value.split_once('%') {
                        entry.exec = left.trim_matches('"').to_owned();
                    } else {
                        entry.exec = value.trim_matches('"').to_owned();
                    }
                }

                _ => {
                    continue;
                }
            }
        }

        if entry.name.is_empty() || entry.exec.is_empty() {
            return None;
        }

        Some(entry)
    }

    fn init() -> Entry {
        Entry {
            name: String::new(),
            exec: String::new(),
        }
    }
}
