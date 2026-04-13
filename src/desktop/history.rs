use std::{
    collections::HashMap,
    fs,
    io::Write,
    ops::{Index, IndexMut},
    path::PathBuf,
};

pub struct EntryHistory {
    entries: HashMap<Box<str>, u32>,
    file_path: PathBuf,
}

impl EntryHistory {
    pub fn parse() -> Result<EntryHistory, std::io::Error> {
        let mut history = EntryHistory {
            entries: HashMap::new(),
            file_path: Self::get_history_file()?,
        };

        history.parse_file()?;

        Ok(history)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut file = fs::File::create(&self.file_path)?;

        for (name, usage) in &self.entries {
            writeln!(file, "{usage} {name}")?;
        }

        Ok(())
    }

    fn parse_file(&mut self) -> std::io::Result<()> {
        let content = std::fs::read_to_string(&self.file_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((num, name)) = line.split_once(' ')
                && let Ok(n) = num.parse::<u32>()
            {
                self.entries.insert(name.trim().into(), n);
            }
        }

        Ok(())
    }

    fn get_history_file() -> Result<PathBuf, std::io::Error> {
        let home = std::env::var("HOME").expect("$HOME is not in UTF-8");
        let cache_home =
            std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{home}/.cache"));

        let path = PathBuf::from(format!("{cache_home}/kedesktop/kelaunch"));

        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        if !path.exists() {
            fs::File::create(&path)?;
        }

        Ok(path)
    }
}

impl Index<&str> for EntryHistory {
    type Output = u32;

    fn index(&self, name: &str) -> &u32 {
        self.entries.get(name).unwrap_or(&0)
    }
}

impl IndexMut<&str> for EntryHistory {
    fn index_mut(&mut self, name: &str) -> &mut u32 {
        if !self.entries.contains_key(name) {
            self.entries.insert(Box::from(name), 0);
        }
        self.entries.get_mut(name).unwrap()
    }
}
