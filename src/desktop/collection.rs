use super::entry::Entry;
use crate::desktop::{EntryHistory, locale::Locale};
use std::{collections::HashSet, ops::Index};

pub struct EntryCollection {
    entries: Vec<Entry>,
}

impl EntryCollection {
    pub fn collect() -> EntryCollection {
        let mut collection = EntryCollection {
            entries: Vec::new(),
        };

        let locale = Locale::from_env();

        for dir in Self::get_applications_dirs() {
            collection.entries.extend(
                walkdir::WalkDir::new(dir)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_type().is_file()
                            && e.path().extension().is_some_and(|ext| ext == "desktop")
                    })
                    .filter_map(|e| Entry::from_file(e.path(), &locale)),
            );
        }

        collection
    }

    pub fn search(&self, query: &str, history: &EntryHistory, out: &mut Vec<(usize, usize)>) {
        out.clear();

        if query.is_empty() {
            out.extend((0..self.entries.len()).map(|i| (i, 0)));
            out.sort_by_key(|&(i, _)| std::cmp::Reverse(history[self.entries[i].get_name()]));
            return;
        }

        let q = query.to_lowercase();
        let mut seen: HashSet<&str> = HashSet::new();
        for (i, e) in self.entries.iter().enumerate() {
            if !seen.insert(e.get_name()) {
                continue;
            }

            let best_span = std::iter::once(e.get_localized_name())
                .chain(e.get_keywords().iter().map(|k| k.as_ref()))
                .filter_map(|s| match_span(s, &q))
                .min();

            if let Some(span) = best_span {
                let usage = history[e.get_name()];

                /* NOTE:
                 * on TIGHTNESS_WEIGHT lower  IS better
                 * on USAGE_WEIGHT     higher IS better
                 */
                const TIGHTNESS_WEIGHT: usize = 100;
                const USAGE_WEIGHT: usize = 30;

                let score = (span * TIGHTNESS_WEIGHT).saturating_sub(usage as usize * USAGE_WEIGHT);
                out.push((i, score));
            }
        }

        out.sort_by_key(|(_, span)| *span);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
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
            .filter(|d| std::fs::metadata(d).is_ok_and(|md| md.is_dir()))
            .collect()
    }
}

impl Index<usize> for EntryCollection {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

fn match_span(haystack: &str, query: &str) -> Option<usize> {
    if haystack.trim().is_empty() {
        return None;
    }

    let mut query_chars = query.chars().peekable();
    let mut start: Option<usize> = None;
    let mut end = 0;

    for (i, hc) in haystack.chars().enumerate() {
        let Some(&qc) = query_chars.peek() else {
            break;
        };

        if hc.to_lowercase().next().unwrap_or(hc) == qc.to_lowercase().next().unwrap_or(qc) {
            if start.is_none() {
                start = Some(i);
            }
            end = i;
            query_chars.next();
        }
    }

    if query_chars.peek().is_none() {
        Some(end - start.unwrap_or(0))
    } else {
        None
    }
}
