# Smart Clipboard

A lightweight clipboard history tracker for Linux. Keeps the last 10 copied items (text and images) and makes them accessible from a system tray menu. History persists across restarts via `~/.clipboard_history.json`.

## Features

- System tray icon with the 10 most recent entries
- Click an entry to copy it back to the clipboard
- Image support — screenshots and copied images are saved as PNGs
- Clear history from the tray or command line
- Runs as a background daemon by default
- CLI for browsing and copying history without the tray

## Requirements

- Linux desktop with a StatusNotifierItem-compatible system tray
- Rust toolchain for building from source

## Install

```bash
cargo build --release
cp target/release/clipboard ~/.local/bin/
```

## Usage

### Tray (default)

Start the tray daemon:

```bash
clipboard
```

Run in the foreground:

```bash
clipboard tray --daemon false
```

### CLI commands

```bash
clipboard list              # list history (newest first)
clipboard list --json       # JSON output
clipboard list --limit 5    # show only last 5
clipboard last              # print most recent entry
clipboard copy 1            # copy entry by index
clipboard clear             # clear history
clipboard path              # show history file path
```

### Tray menu

The tray shows up to 10 entries, newest first. Click any entry to copy it back to the clipboard. Use the tray menu to clear history or open the history file.

## Data storage

| Data | Location |
|------|----------|
| History | `~/.clipboard_history.json` |
| Images | `~/.clipboard_images/*.png` |

## Notes

- Tested on Linux with KDE, GNOME, and other StatusNotifierItem-compatible trays.
- To autostart with your desktop, add `clipboard` to your startup applications.
