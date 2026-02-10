use arboard::Clipboard;
use clap::{ Parser, Subcommand };
use clipboard::Clip;
use daemonize::Daemonize;
use ksni::{ menu::{ MenuItem, StandardItem, SubMenu }, Tray, TrayService, ToolTip };
use std::{ fs::File, path::PathBuf, process::Command as ProcessCommand, thread, time::Duration };

#[derive(Parser)]
#[command(name = "clipboard", version, about = "Smart clipboard history tracker")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List saved entries
    List {
        /// Show output as JSON
        #[arg(long)]
        json: bool,
        /// Limit number of entries (newest first)
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Print the most recent entry
    Last {
        /// Show output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Copy an entry back to the clipboard by index from `list`
    Copy {
        /// 1-based index from the list output
        index: usize,
    },
    /// Clear saved history
    Clear,
    /// Show the history file path
    Path,
    /// Run a system tray icon with recent entries
    Tray {
        /// Polling interval in milliseconds
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
        /// Run in the background (default true)
        #[arg(long, default_value_t = true)]
        daemon: bool,
        /// Optional PID file path (daemon mode)
        #[arg(long)]
        pid_file: Option<PathBuf>,
        /// Optional log file path (daemon mode)
        #[arg(long)]
        log_file: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Tray {
        interval_ms: 1000,
        daemon: true,
        pid_file: None,
        log_file: None,
    });

    match command {
        Command::List { json, limit } => list_entries(json, limit),
        Command::Last { json } => print_last(json),
        Command::Copy { index } => copy_entry(index),
        Command::Clear => clear_entries(),
        Command::Path => println!("{}", Clip::file_path().display()),
        Command::Tray { interval_ms, daemon, pid_file, log_file } => {
            if let Err(err) = run_tray(interval_ms, daemon, pid_file, log_file) {
                eprintln!("{err}");
            }
        }
    }
}

#[derive(Clone)]
struct TrayEntry {
    text: String,
    timestamp: Option<String>,
}

struct ClipboardTray {
    entries: Vec<TrayEntry>,
}

impl ClipboardTray {
    fn from_clip(clip: &Clip) -> Self {
        Self {
            entries: clip
                .entries()
                .iter()
                .map(|entry| TrayEntry {
                    text: entry.text().to_string(),
                    timestamp: entry.timestamp().map(|ts| ts.to_string()),
                })
                .collect(),
        }
    }

    fn update_from_clip(&mut self, clip: &Clip) {
        self.entries = clip
            .entries()
            .iter()
            .map(|entry| TrayEntry {
                text: entry.text().to_string(),
                timestamp: entry.timestamp().map(|ts| ts.to_string()),
            })
            .collect();
    }
}

impl Tray for ClipboardTray {
    fn id(&self) -> String {
        "smart-clipboard".to_string()
    }

    fn title(&self) -> String {
        "Smart Clipboard".to_string()
    }

    fn icon_name(&self) -> String {
        "edit-paste".to_string()
    }

    fn tool_tip(&self) -> ToolTip {
        let count = self.entries.len();
        let last = self.entries.last().map(|entry| {
            let label = sanitize_label(&entry.text);
            let label = truncate_label(&label, 60);
            let ts = entry.timestamp.as_deref().unwrap_or("-");
            format!("{ts} • {label}")
        });

        ToolTip {
            title: "Smart Clipboard".to_string(),
            description: match last {
                Some(last) => format!("{count} items\nLast: {last}"),
                None => "No entries".to_string(),
            },
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = Vec::new();

        let mut entry_items: Vec<MenuItem<Self>> = Vec::new();
        for (idx, entry) in self.entries.iter().rev().enumerate() {
            let label = sanitize_label(&entry.text);
            let label = truncate_label(&label, 80);
            let text = entry.text.clone();
            let display = format!("{:>2}. {label}", idx + 1);

            entry_items.push(
                (StandardItem {
                    label: display,
                    activate: Box::new(move |_| {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            let _ = clipboard.set_text(text.clone());
                        }
                    }),
                    ..Default::default()
                }).into()
            );
        }

        if entry_items.is_empty() {
            entry_items.push(
                (StandardItem {
                    label: "No entries yet".to_string(),
                    enabled: false,
                    ..Default::default()
                }).into()
            );
        }

        items.push(
            (SubMenu {
                label: "Recent entries".to_string(),
                submenu: entry_items,
                ..Default::default()
            }).into()
        );

        items.push(MenuItem::Separator);

        items.push(
            (StandardItem {
                label: "Open history file".to_string(),
                activate: Box::new(|_| {
                    let path = Clip::file_path();
                    let _ = ProcessCommand::new("xdg-open").arg(path).spawn();
                }),
                ..Default::default()
            }).into()
        );

        items.push(
            (StandardItem {
                label: "Clear history".to_string(),
                activate: Box::new(|tray: &mut ClipboardTray| {
                    let mut clip = Clip::load();
                    clip.clear();
                    clip.save();
                    tray.entries.clear();
                }),
                ..Default::default()
            }).into()
        );

        items.push(
            (StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }).into()
        );

        items
    }
}

fn run_tray(
    interval_ms: u64,
    daemon: bool,
    pid_file: Option<PathBuf>,
    log_file: Option<PathBuf>
) -> Result<(), String> {
    if daemon {
        let pid_path = pid_file.unwrap_or_else(default_pid_path);
        let log_path = log_file.unwrap_or_else(default_log_path);

        let stdout = File::create(&log_path).map_err(|e|
            format!("Failed to create log file {}: {e}", log_path.display())
        )?;
        let stderr = File::create(&log_path).map_err(|e|
            format!("Failed to create log file {}: {e}", log_path.display())
        )?;

        let daemonize = Daemonize::new().pid_file(&pid_path).stdout(stdout).stderr(stderr);
        daemonize.start().map_err(|err| format!("Failed to daemonize: {err}"))?;
    }

    let mut clip = Clip::load();
    let tray = ClipboardTray::from_clip(&clip);
    let service = TrayService::new(tray);
    let handle = service.handle();
    service.spawn();

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;

    loop {
        let last = clipboard.get_text().unwrap_or_default();
        if !last.is_empty() && clip.should_add(&last) {
            clip.add(&last);
            clip.save();
            handle.update(|tray| tray.update_from_clip(&clip));
        }
        thread::sleep(Duration::from_millis(interval_ms));
    }
}

fn sanitize_label(text: &str) -> String {
    text.replace('\n', " ").replace('\r', " ").replace('_', "__")
}

fn truncate_label(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if count >= max_chars {
            out.push('…');
            return out;
        }
        out.push(ch);
        count += 1;
    }
    out
}

fn list_entries(json: bool, limit: Option<usize>) {
    let clip = Clip::load();
    if json {
        println!("{}", serde_json::to_string_pretty(&clip).unwrap_or_default());
        return;
    }

    let entries = clip.entries();
    let iter = entries.iter().rev().enumerate();
    let mut count = 0usize;
    for (i, entry) in iter {
        if let Some(limit) = limit {
            if count >= limit {
                break;
            }
        }
        let index = i + 1;
        let ts = entry.timestamp().unwrap_or("-");
        println!("{index}\t{ts}\t{}", entry.text());
        count += 1;
    }
}

fn print_last(json: bool) {
    let clip = Clip::load();
    if let Some(entry) = clip.last() {
        if json {
            println!("{}", serde_json::to_string_pretty(entry).unwrap_or_default());
        } else {
            let ts = entry.timestamp().unwrap_or("-");
            println!("{ts}\t{}", entry.text());
        }
    }
}

fn copy_entry(index: usize) {
    if index == 0 {
        eprintln!("Index must be >= 1");
        return;
    }

    let clip = Clip::load();
    let entry = clip
        .entries()
        .iter()
        .rev()
        .nth(index - 1);
    match entry {
        Some(entry) =>
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    if let Err(err) = clipboard.set_text(entry.text().to_string()) {
                        eprintln!("Failed to set clipboard: {err}");
                    }
                }
                Err(err) => eprintln!("Failed to access clipboard: {err}"),
            }
        None => eprintln!("No entry at index {index}"),
    }
}

fn clear_entries() {
    let mut clip = Clip::load();
    clip.clear();
    clip.save();
}

fn default_pid_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".clipboard_history.pid");
    path
}

fn default_log_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".clipboard_history.log");
    path
}
