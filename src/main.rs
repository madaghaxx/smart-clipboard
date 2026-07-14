use arboard::Clipboard;
use clap::{Parser, Subcommand};
use clipboard::Clip;
use daemonize::Daemonize;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use ksni::{
    menu::{MenuItem, StandardItem},
    ToolTip, Tray, TrayService,
};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{
    fs::{self, File},
    io::BufWriter,
    path::PathBuf,
    process::Command as ProcessCommand,
    thread,
    time::Duration,
};

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
        Command::Tray {
            interval_ms,
            daemon,
            pid_file,
            log_file,
        } => {
            if let Err(err) = run_tray(interval_ms, daemon, pid_file, log_file) {
                eprintln!("{err}");
            }
        }
    }
}

#[derive(Clone)]
struct TrayEntry {
    text: String,
    is_image: bool,
}

struct ClipboardTray {
    entries: Vec<TrayEntry>,
    reload_flag: Arc<AtomicBool>,
}

impl ClipboardTray {
    fn from_clip(clip: &Clip, reload_flag: Arc<AtomicBool>) -> Self {
        Self {
            entries: clip
                .entries()
                .iter()
                .map(|entry| TrayEntry {
                    text: entry.image_file().unwrap_or(entry.text()).to_string(),
                    is_image: entry.is_image(),
                })
                .collect(),
            reload_flag,
        }
    }

    fn update_from_clip(&mut self, clip: &Clip) {
        self.entries = clip
            .entries()
            .iter()
            .map(|entry| TrayEntry {
                text: entry.image_file().unwrap_or(entry.text()).to_string(),
                is_image: entry.is_image(),
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
            let label = if entry.is_image {
                "[Image]".to_string()
            } else {
                sanitize_label(&entry.text)
            };
            truncate_label(&label, 60)
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

        if self.entries.is_empty() {
            items.push(
                (StandardItem {
                    label: "No entries yet".to_string(),
                    enabled: false,
                    ..Default::default()
                })
                .into(),
            );
        } else {
            for (idx, entry) in self.entries.iter().rev().enumerate() {
                let text = entry.text.clone();
                let is_image = entry.is_image;
                let preview = if is_image {
                    "[Image]".to_string()
                } else {
                    format_entry_preview(&entry.text)
                };
                let display = format!("{:>2}. {preview}", idx + 1);

                items.push(
                    (StandardItem {
                        label: display,
                        activate: Box::new(move |_| {
                            if is_image {
                                if let Ok(mut clipboard) = Clipboard::new() {
                                    if let Some(img_data) = load_image_file(&text) {
                                        let _ = clipboard.set_image(img_data);
                                    }
                                }
                            } else if let Ok(mut clipboard) = Clipboard::new() {
                                let _ = clipboard.set_text(text.clone());
                            }
                            let _ = ProcessCommand::new("notify-send")
                                .args(["Smart Clipboard", "Copied to clipboard"])
                                .spawn();
                        }),
                        ..Default::default()
                    })
                    .into(),
                );
            }
        }

        items.push(MenuItem::Separator);

        items.push(
            (StandardItem {
                label: "Open history file".to_string(),
                activate: Box::new(|_| {
                    let path = Clip::file_path();
                    let _ = ProcessCommand::new("xdg-open").arg(path).spawn();
                }),
                ..Default::default()
            })
            .into(),
        );

        items.push(
            (StandardItem {
                label: "Clear history".to_string(),
                activate: Box::new(|tray: &mut ClipboardTray| {
                    let mut clip = Clip::load();
                    clip.clear();
                    clip.save();
                    tray.entries.clear();
                    tray.reload_flag.store(true, Ordering::SeqCst);
                }),
                ..Default::default()
            })
            .into(),
        );

        items.push(
            (StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            })
            .into(),
        );

        items
    }
}

fn run_tray(
    interval_ms: u64,
    daemon: bool,
    pid_file: Option<PathBuf>,
    log_file: Option<PathBuf>,
) -> Result<(), String> {
    if daemon {
        let pid_path = pid_file.unwrap_or_else(default_pid_path);
        let log_path = log_file.unwrap_or_else(default_log_path);

        let stdout = File::create(&log_path)
            .map_err(|e| format!("Failed to create log file {}: {e}", log_path.display()))?;
        let stderr = File::create(&log_path)
            .map_err(|e| format!("Failed to create log file {}: {e}", log_path.display()))?;

        let daemonize = Daemonize::new()
            .pid_file(&pid_path)
            .stdout(stdout)
            .stderr(stderr);
        daemonize
            .start()
            .map_err(|err| format!("Failed to daemonize: {err}"))?;
    }

    let mut clip = Clip::load();
    let reload_flag = Arc::new(AtomicBool::new(false));
    let tray = ClipboardTray::from_clip(&clip, reload_flag.clone());
    let service = TrayService::new(tray);
    let handle = service.handle();
    service.spawn();

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;
    let mut last_image_hash: Option<String> = None;

    loop {
        if reload_flag.load(Ordering::SeqCst) {
            clip = Clip::load();
            reload_flag.store(false, Ordering::SeqCst);
            handle.update(|tray| tray.update_from_clip(&clip));
        }

        let text = clipboard.get_text().unwrap_or_default();
        if !text.is_empty() {
            if clip.should_add_text(&text) {
                clip.add_text(&text);
                clip.save();
                handle.update(|tray| tray.update_from_clip(&clip));
            }
        } else {
            if let Ok(image) = clipboard.get_image() {
                let hash = hash_image_bytes(&image.bytes);
                let is_new = last_image_hash.as_deref() != Some(&hash);
                last_image_hash = Some(hash.clone());
                let filename = format!("{}.png", hash);

                if is_new && clip.should_add_image(&filename) {
                    let path = Clip::images_dir().join(&filename);
                    let file = File::create(&path).ok();
                    if let Some(file) = file {
                        let writer = BufWriter::new(file);
                        let encoder = PngEncoder::new(writer);
                        if encoder
                            .write_image(
                                &image.bytes,
                                image.width as u32,
                                image.height as u32,
                                ExtendedColorType::Rgba8,
                            )
                            .is_ok()
                        {
                            clip.add_image(&filename);
                            clip.save();
                            handle.update(|tray| tray.update_from_clip(&clip));
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(interval_ms));
    }
}

fn sanitize_label(text: &str) -> String {
    text.replace(['\n', '\r'], " ").replace('_', "__")
}

fn truncate_label(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (count, ch) in text.chars().enumerate() {
        if count >= max_chars {
            out.push('…');
            return out;
        }
        out.push(ch);
    }
    out
}

fn format_entry_preview(text: &str) -> String {
    let line_count = text.lines().count();
    let first_line = text.lines().next().unwrap_or("");
    let sanitized = sanitize_label(first_line);
    if line_count > 1 {
        let truncated = truncate_label(&sanitized, 47);
        let cleaned = truncated.trim_end_matches('…').trim_end();
        format!("{} …", cleaned)
    } else {
        truncate_label(&sanitized, 50).trim_end().to_string()
    }
}

fn hash_image_bytes(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn load_image_file(filename: &str) -> Option<arboard::ImageData<'static>> {
    let path = Clip::images_dir().join(filename);
    let data = fs::read(&path).ok()?;
    let dynamic = image::load_from_memory(&data).ok()?;
    let rgba = dynamic.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    })
}

fn list_entries(json: bool, limit: Option<usize>) {
    let clip = Clip::load();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&clip).unwrap_or_default()
        );
        return;
    }

    for (i, entry) in clip.entries().iter().rev().enumerate() {
        if let Some(limit) = limit {
            if i >= limit {
                break;
            }
        }
        let index = i + 1;
        let content = if entry.is_image() {
            "[Image]".to_string()
        } else {
            entry.text().to_string()
        };
        println!("{index}\t{content}");
    }
}

fn print_last(json: bool) {
    let clip = Clip::load();
    if let Some(entry) = clip.last() {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(entry).unwrap_or_default()
            );
        } else {
            let content = if entry.is_image() {
                "[Image]".to_string()
            } else {
                entry.text().to_string()
            };
            println!("{content}");
        }
    }
}

fn copy_entry(index: usize) {
    if index == 0 {
        eprintln!("Index must be >= 1");
        return;
    }

    let clip = Clip::load();
    let entry = clip.entries().iter().rev().nth(index - 1);
    match entry {
        Some(entry) => {
            let result = match Clipboard::new() {
                Ok(mut clipboard) => {
                    if entry.is_image() {
                        if let Some(filename) = entry.image_file() {
                            if let Some(img_data) = load_image_file(filename) {
                                clipboard.set_image(img_data).map_err(|e| e.to_string())
                            } else {
                                Err("Failed to load image file".to_string())
                            }
                        } else {
                            Err("Image entry missing filename".to_string())
                        }
                    } else {
                        clipboard
                            .set_text(entry.text().to_string())
                            .map_err(|e| e.to_string())
                    }
                }
                Err(err) => Err(format!("Failed to access clipboard: {err}")),
            };
            if let Err(err) = result {
                eprintln!("{err}");
            }
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
