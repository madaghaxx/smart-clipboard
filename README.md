## What is it?
This a light smart clipboard that stores your last (for now it is 10) copied items.
It is written in Rust and it uses the following crates: Chrono,serde and serde_json, and abroad.
It is very usefull especially for developers and coders.
It counters restarts and crashing and prevent data lose by saving it in a `~/.clipboard_history.json`.
The binary will not consume much because it has a 30sec timer and only display when something new is added.
### Instructions:
##### Important: I have only tested it on linux for now.
You can either install rust and then use:
```zsh
cargo add arboard serde serde_json chrono
```
 Or download the binary directly:

[Download last version](https://github.com/madaghaxx/smart-clipboard/releases/tag/app)

### Contribution:
You are free to add anything.