# Smart Clipboard

A lightweight clipboard history for Linux that keeps the last 10 copied items and makes them available from a system tray menu. It persists history across restarts and crashes by saving to ~/.clipboard_history.json.

## Features

- System tray with the 10 most recent entries
- Click an entry to copy it back to the clipboard
- Clear history from the tray
- Runs in the background by default
- Stores timestamps for each entry

## Requirements

- Linux desktop with a StatusNotifierItem-compatible system tray
- Rust toolchain for building from source

## Install (from source)

Build a release binary:

```bash
cargo build --release
```

Run the binary directly:

```bash
./target/release/clipboard
```

Optional: put it on PATH:

```bash
cp target/release/clipboard ~/.local/bin/
clipboard
```

## Usage

The default behavior starts the tray in the background (daemonized), so closing the terminal won’t stop it:

```bash
clipboard
```

Run in the foreground:

```bash
clipboard tray --daemon false
```

Show the history file path:

```bash
clipboard path
```

List entries in the terminal:

```bash
clipboard list
```

Copy an entry by index (from list output):

```bash
clipboard copy 1
```

Clear history:

```bash
clipboard clear
```

## Examples

Tray menu example (newest first):

```text
Recent entries
	1. cargo build --release
	2. git status
	3. Hello world
```

Terminal list output:

```text
1  2026-02-10T10:35:18+00:00  cargo build --release
2  2026-02-10T10:35:12+00:00  git status
3  2026-02-10T10:34:58+00:00  Hello world
```

## Data storage

History is stored at:

```text
~/.clipboard_history.json
```

## Notes

- Tested on Linux. If the tray icon does not appear, ensure your desktop environment supports StatusNotifierItem trays.

## Contributing

Pull requests are welcome.
