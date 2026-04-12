use crate::desktop::EntryHistory;

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
                    .follow_links(true)
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

    pub fn search(&self, query: &str, history: Option<&EntryHistory>) -> Vec<&Entry> {
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

                let best_span = std::iter::once(name.as_str())
                    .chain(e.keywords.iter().map(|k| k.as_str()))
                    .filter_map(|s| match_span(&s.to_lowercase(), &q))
                    .min();

                best_span.map(|span| {
                    let usage = history.map(|h| h[&e.name]).unwrap_or(0);

                    /* note: tweak to increase which matters more */
                    const TIGHTNESS_WEIGHT: usize = 2;
                    const USAGE_WEIGHT: usize = 0;

                    let score =
                        (span * TIGHTNESS_WEIGHT).saturating_sub(usage as usize * USAGE_WEIGHT);
                    (e, score)
                })
            })
            .collect();

        results.sort_by_key(|(_, span)| *span);
        results.into_iter().map(|(e, _)| e).collect()
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        for entry in &self.entries {
            if entry.name == name {
                return Some(&entry);
            }
        }

        None
    }

    fn get_applications_dirs() -> Vec<String> {
        let home = std::env::var("HOME").expect("$HOME is not in UTF-8");

        let data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());

        let data_home =
            std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));

        let mut dirs: Vec<String> = std::iter::once(data_home.as_str())
            .chain(data_dirs.split(':'))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect();

        dirs.push("/var/lib/flatpak/exports/share".to_owned());
        dirs.push(format!("{home}/.local/share/flatpak/exports/share"));

        let mut seen = std::collections::HashSet::new();
        dirs.into_iter()
            .filter(|d| seen.insert(d.clone()))
            .map(|d| format!("{d}/applications"))
            .filter(|d| std::fs::metadata(d).map_or(false, |md| md.is_dir()))
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
