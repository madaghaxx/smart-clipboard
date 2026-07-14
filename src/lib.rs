use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{fs, path::PathBuf};

const MAX_ENTRIES: usize = 10;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Clip {
    saved: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_file: Option<String>,
}

impl Entry {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_image(&self) -> bool {
        self.image_file.is_some()
    }

    pub fn image_file(&self) -> Option<&str> {
        self.image_file.as_deref()
    }
}

impl Clip {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_text(&mut self, text: &str) {
        if self.saved.len() >= MAX_ENTRIES {
            self.remove_entry(0);
        }
        self.saved.push(Entry {
            text: text.to_string(),
            image_file: None,
        });
    }

    pub fn add_image(&mut self, image_file: &str) {
        if self.saved.len() >= MAX_ENTRIES {
            self.remove_entry(0);
        }
        self.saved.push(Entry {
            text: String::new(),
            image_file: Some(image_file.to_string()),
        });
    }

    fn remove_entry(&mut self, index: usize) {
        if let Some(file) = self.saved.remove(index).image_file {
            let path = Self::images_dir().join(&file);
            let _ = fs::remove_file(path);
        }
    }

    pub fn should_add_text(&self, text: &str) -> bool {
        self.saved
            .last()
            .is_none_or(|last| last.is_image() || last.text != text)
    }

    pub fn should_add_image(&self, image_file: &str) -> bool {
        self.saved
            .last()
            .is_none_or(|last| last.image_file.as_deref() != Some(image_file))
    }

    pub fn entries(&self) -> &[Entry] {
        &self.saved
    }

    pub fn last(&self) -> Option<&Entry> {
        self.saved.last()
    }

    pub fn clear(&mut self) {
        let unique: HashSet<&str> = self
            .saved
            .iter()
            .filter_map(|e| e.image_file.as_deref())
            .collect();
        for file in unique {
            let path = Self::images_dir().join(file);
            let _ = fs::remove_file(path);
        }
        self.saved.clear();
    }

    pub fn save(&self) {
        let path = Self::file_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn load() -> Self {
        let path = Self::file_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(clip) = serde_json::from_str::<Clip>(&data) {
                return clip;
            }
        }
        Clip::new()
    }

    pub fn file_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".clipboard_history.json");
        path
    }

    pub fn images_dir() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".clipboard_images");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path
    }
}
