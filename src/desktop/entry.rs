use std::path::Path;

#[derive(Debug)]
pub struct Entry {
    pub name: Box<str>,
    pub exec: Box<str>,
    pub keywords: Box<[Box<str>]>,

    pub is_term: bool,
}

impl Entry {
    pub fn from_file(desktop_file_path: &Path) -> Option<Entry> {
        let content = std::fs::read_to_string(desktop_file_path).ok()?;
        let mut in_desktop_entry = false;

        let mut name: Option<Box<str>> = None;
        let mut exec: Option<Box<str>> = None;
        let mut keywords: Option<Box<[Box<str>]>> = None;
        let mut is_term: bool = false;

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

            if key.contains('[') {
                continue;
            }

            let key = key.trim();
            let value = value.trim();

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
                    keywords = Some(
                        value
                            .split(';')
                            .map(|s| Box::from(s.trim().to_lowercase()))
                            .collect::<Vec<Box<str>>>()
                            .into_boxed_slice(),
                    );
                }
                "Terminal" => is_term = value == "true",
                _ => {}
            }
        }

        let (Some(name), Some(exec)) = (name, exec) else {
            return None;
        };

        if name.is_empty() || !Self::is_exec_valid(&exec) {
            return None;
        }

        Some(Entry {
            name,
            exec,
            keywords: keywords.unwrap_or_default(),
            is_term,
        })
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
}

fn strip_field_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
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
        } else {
            result.push(c);
        }
    }

    result
}
