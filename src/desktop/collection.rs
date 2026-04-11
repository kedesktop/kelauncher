use std::{env, fs};

use super::entry::Entry;

pub struct EntryCollection {
    entries: Vec<Entry>,
}

impl EntryCollection {
    pub fn collect() -> EntryCollection {
        let mut collection = EntryCollection {
            entries: Vec::new(),
        };

        for dir in Self::get_applications_dirs() {
            collection.entries.extend(
                walkdir::WalkDir::new(dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_type().is_file()
                            && e.path().extension().map_or(false, |ext| ext == "desktop")
                    })
                    .filter_map(|e| Entry::from_file(e.path())),
            );
        }

        collection
    }

    pub fn search(&self, query: &str) -> Vec<&Entry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }

        let q = query.to_lowercase();
        let mut seen = std::collections::HashSet::new();
        let mut results: Vec<(&Entry, usize)> = self
            .entries
            .iter()
            .filter_map(|e| {
                let name = e.name.to_lowercase();
                if !seen.insert(name.clone()) {
                    return None;
                }
                match_span(&name, &q).map(|span| (e, span))
            })
            .collect();

        results.sort_by_key(|(_, span)| *span);
        results.into_iter().map(|(e, _)| e).collect()
    }

    fn get_applications_dirs() -> Vec<String> {
        if let (Ok(xdg_dirs), Ok(home_dirs)) =
            (env::var("XDG_DATA_DIRS"), env::var("XDG_DATA_HOME"))
        {
            format!("{xdg_dirs}:{home_dirs}")
        } else {
            format!(
                "/usr/share:/usr/local/share:{}/.local/share",
                env::var("HOME").expect("$HOME is not in UTF-8")
            )
        }
        .split(':')
        .filter(|s| !s.is_empty() && fs::metadata(s).map_or(false, |md| md.is_dir()))
        .map(str::to_owned)
        .collect()
    }
}

fn match_span(haystack: &str, query: &str) -> Option<usize> {
    if haystack.trim().is_empty() {
        return None;
    }

    let haystack: Vec<char> = haystack.chars().collect();
    let query: Vec<char> = query.chars().collect();

    let mut qi = 0;
    let mut start = None;
    let mut end = 0;

    for (i, &h) in haystack.iter().enumerate() {
        if qi < query.len() && h == query[qi] {
            if start.is_none() {
                start = Some(i);
            }
            end = i;
            qi += 1;
        }
    }

    if qi == query.len() {
        Some(end - start.unwrap_or(0))
    } else {
        None
    }
}
