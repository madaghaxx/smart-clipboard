use chrono::{ DateTime, Local };
use serde::{ Deserialize, Serialize };
use std::{ fs, path::PathBuf };

const MAX_ENTRIES: usize = 10;
#[derive(Serialize, Deserialize, Debug)]
pub struct Clip {
    saved: Vec<Entry>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    text: String,
    #[serde(default)]
    timestamp: Option<String>,
}
impl Entry {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn timestamp(&self) -> Option<&str> {
        self.timestamp.as_deref()
    }
}
impl Clip {
    pub fn new() -> Self {
        Clip {
            saved: Vec::new(),
        }
    }
    pub fn add(&mut self, ele: &str) {
        if self.saved.len() >= MAX_ENTRIES {
            self.saved.remove(0);
        }
        self.saved.push(Entry {
            text: ele.to_string(),
            timestamp: Some(Self::now_string()),
        });
    }
    pub fn should_add(&self, text: &str) -> bool {
        self.saved.last().map_or(true, |last| last.text != text)
    }
    pub fn entries(&self) -> &[Entry] {
        &self.saved
    }
    pub fn last(&self) -> Option<&Entry> {
        self.saved.last()
    }
    pub fn clear(&mut self) {
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
    fn now_string() -> String {
        let now: DateTime<Local> = Local::now();
        now.to_rfc3339()
    }
}
