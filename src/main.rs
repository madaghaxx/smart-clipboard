use arboard::Clipboard;
use std::{ thread, time::Duration };
use clipboard::*;
fn main() {
    let wait_time = 3000;
    let mut lasts = Clip::load(); // load history from file if exists
    let mut clipboard = Clipboard::new().expect("Failed to access clipboard");

    loop {
        let last = clipboard.get_text().unwrap_or_default();

        if !last.is_empty() && lasts.should_add(&last) {
            lasts.add(&last);
            lasts.save(); // persist history
            println!("{}", serde_json::to_string_pretty(&lasts).unwrap());
        }

        thread::sleep(Duration::from_millis(wait_time));
    }
}
